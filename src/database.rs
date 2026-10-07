use std::fmt::Write;
use std::fs::{DirBuilder, OpenOptions};
use std::path::{Path, PathBuf};

use etcetera::BaseStrategy;
use rusqlite::{Connection, ErrorCode};
use secrecy::{ExposeSecret, ExposeSecretMut, SecretBox, SecretString};

use crate::{Error, Result};

const APP_DIR: &str = "iopole";
const DATABASE_FILE: &str = "iopole.db";
const KEY_BYTES: usize = 32;
const SQLITE_SIDE_FILE_SUFFIXES: [&str; 3] = ["-journal", "-wal", "-shm"];

const MIGRATIONS: [&str; 1] = [include_str!("migrations/001_invoices.sql")];

pub(crate) trait DatabaseKeyStore {
    fn load_key(&self) -> Result<Option<SecretString>>;
    fn save_key(&self, key: &SecretString) -> Result<()>;
    fn delete_key(&self) -> Result<()>;
}

pub(crate) fn default_path() -> Result<PathBuf> {
    let data_dir = etcetera::choose_base_strategy()?.data_dir();
    Ok(data_dir.join(APP_DIR).join(DATABASE_FILE))
}

pub(crate) fn open(path: &Path, keys: &impl DatabaseKeyStore) -> Result<Connection> {
    let key = match keys.load_key()? {
        Some(key) => key,
        None if path.exists() => return Err(Error::MissingDatabaseKey(path.to_owned())),
        None => {
            let key = generate_key()?;
            keys.save_key(&key)?;
            key
        }
    };

    if let Some(dir) = path.parent() {
        create_private_dir(dir)?;
    }
    create_private_file(path)?;

    let mut connection = Connection::open(path)?;
    unlock(&connection, &key, path)?;
    migrate(&mut connection)?;
    Ok(connection)
}

pub(crate) fn delete(path: &Path, keys: &impl DatabaseKeyStore) -> Result<()> {
    remove_if_exists(path)?;
    for suffix in SQLITE_SIDE_FILE_SUFFIXES {
        let mut side_file = path.as_os_str().to_owned();
        side_file.push(suffix);
        remove_if_exists(Path::new(&side_file))?;
    }
    keys.delete_key()
}

fn remove_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

fn generate_key() -> Result<SecretString> {
    let mut bytes = SecretBox::new(Box::new([0_u8; KEY_BYTES]));
    getrandom::fill(bytes.expose_secret_mut())?;

    let mut hex = String::with_capacity(KEY_BYTES * 2);
    for byte in bytes.expose_secret() {
        let _ = write!(hex, "{byte:02x}");
    }
    Ok(SecretString::from(hex))
}

fn unlock(connection: &Connection, key: &SecretString, path: &Path) -> Result<()> {
    let raw_key = SecretString::from(format!("x'{}'", key.expose_secret()));
    connection.pragma_update(None, "key", raw_key.expose_secret())?;

    match connection.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(())) {
        Ok(()) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(err, _)) if err.code == ErrorCode::NotADatabase => {
            Err(Error::InvalidDatabaseKey(path.to_owned()))
        }
        Err(err) => Err(err.into()),
    }
}

fn migrate(connection: &mut Connection) -> Result<()> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    let applied = usize::try_from(version)
        .ok()
        .filter(|&applied| applied <= MIGRATIONS.len())
        .ok_or(Error::UnsupportedSchema(version))?;

    for (version, migration) in (1_i64..).zip(MIGRATIONS).skip(applied) {
        let transaction = connection.transaction()?;
        transaction.execute_batch(migration)?;
        transaction.pragma_update(None, "user_version", version)?;
        transaction.commit()?;
    }
    Ok(())
}

fn create_private_dir(dir: &Path) -> std::io::Result<()> {
    let mut builder = DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder.create(dir)
}

fn create_private_file(path: &Path) -> std::io::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(false);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path).map(drop)
}

#[cfg(test)]
pub(crate) mod testing;

#[cfg(test)]
mod tests;
