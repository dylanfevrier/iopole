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

const MIGRATIONS: [&str; 2] = [
    include_str!("migrations/001_invoices.sql"),
    include_str!("migrations/002_party_identifiers.sql"),
];

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

pub(crate) fn create_private_dir(dir: &Path) -> std::io::Result<()> {
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
pub(crate) mod testing {
    use std::cell::RefCell;

    use super::*;

    #[derive(Default)]
    pub(crate) struct InMemoryKeyStore {
        key: RefCell<Option<SecretString>>,
    }

    impl DatabaseKeyStore for InMemoryKeyStore {
        fn load_key(&self) -> Result<Option<SecretString>> {
            Ok(self.key.borrow().clone())
        }

        fn save_key(&self, key: &SecretString) -> Result<()> {
            *self.key.borrow_mut() = Some(key.clone());
            Ok(())
        }

        fn delete_key(&self) -> Result<()> {
            self.key.borrow_mut().take();
            Ok(())
        }
    }

    pub(crate) struct InvoiceRecord<'a> {
        pub(crate) id: &'a str,
        pub(crate) direction: &'a str,
        pub(crate) created_at: &'a str,
        pub(crate) number: Option<&'a str>,
        pub(crate) amount: Option<&'a str>,
        pub(crate) seller_name: Option<&'a str>,
        pub(crate) seller_identifier: Option<&'a str>,
        pub(crate) buyer_name: Option<&'a str>,
        pub(crate) buyer_identifier: Option<&'a str>,
        pub(crate) state: &'a str,
        pub(crate) last_status_code: Option<&'a str>,
        pub(crate) raw: &'a str,
    }

    impl Default for InvoiceRecord<'_> {
        fn default() -> Self {
            Self {
                id: "01a1",
                direction: "INBOUND",
                created_at: "2026-09-01T10:00:00.000+00:00",
                number: None,
                amount: None,
                seller_name: None,
                seller_identifier: None,
                buyer_name: None,
                buyer_identifier: None,
                state: "VALID",
                last_status_code: None,
                raw: "{}",
            }
        }
    }

    impl InvoiceRecord<'_> {
        pub(crate) fn insert(&self, connection: &rusqlite::Connection) -> Result<()> {
            connection.execute(
                "INSERT INTO invoice (invoice_id, direction, created_at, number, amount,
                                  seller_name, seller_identifier, buyer_name, buyer_identifier,
                                  state, last_status_code, raw, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, '')",
                rusqlite::params![
                    self.id,
                    self.direction,
                    self.created_at,
                    self.number,
                    self.amount,
                    self.seller_name,
                    self.seller_identifier,
                    self.buyer_name,
                    self.buyer_identifier,
                    self.state,
                    self.last_status_code,
                    self.raw,
                ],
            )?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::InMemoryKeyStore;
    use super::*;

    fn create_populated_database(path: &Path, keys: &InMemoryKeyStore) -> Result<()> {
        open(path, keys)?
            .execute_batch("CREATE TABLE probe (id INTEGER); INSERT INTO probe VALUES (1)")?;
        Ok(())
    }

    #[test]
    fn creates_database_and_missing_directories() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("nested/iopole.db");

        create_populated_database(&path, &InMemoryKeyStore::default())?;

        assert!(path.is_file());
        Ok(())
    }

    #[test]
    fn stores_generated_key_on_first_open() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let keys = InMemoryKeyStore::default();

        open(&tmp.path().join("iopole.db"), &keys)?;

        let key = keys.load_key()?.map(|key| key.expose_secret().to_owned());
        assert_eq!(key.map(|key| key.len()), Some(KEY_BYTES * 2));
        Ok(())
    }

    #[test]
    fn database_file_is_encrypted() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");

        create_populated_database(&path, &InMemoryKeyStore::default())?;

        let header = std::fs::read(&path)?;
        assert!(!header.starts_with(b"SQLite format 3"));
        Ok(())
    }

    #[test]
    fn reopening_with_same_key_reads_existing_data() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        create_populated_database(&path, &keys)?;

        let count: i64 =
            open(&path, &keys)?.query_row("SELECT count(*) FROM probe", [], |row| row.get(0))?;

        assert_eq!(count, 1);
        Ok(())
    }

    #[test]
    fn rejects_wrong_key() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        create_populated_database(&path, &InMemoryKeyStore::default())?;
        let other_keys = InMemoryKeyStore::default();
        other_keys.save_key(&generate_key()?)?;

        let result = open(&path, &other_keys);

        assert!(matches!(result, Err(Error::InvalidDatabaseKey(_))));
        Ok(())
    }

    #[test]
    fn refuses_existing_database_without_key() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        create_populated_database(&path, &InMemoryKeyStore::default())?;

        let result = open(&path, &InMemoryKeyStore::default());

        assert!(matches!(result, Err(Error::MissingDatabaseKey(_))));
        Ok(())
    }

    #[test]
    fn delete_removes_database_and_key() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        create_populated_database(&path, &keys)?;

        delete(&path, &keys)?;

        assert!(!path.exists());
        assert!(keys.load_key()?.is_none());
        Ok(())
    }

    #[test]
    fn delete_succeeds_when_database_does_not_exist() -> Result<()> {
        let tmp = tempfile::tempdir()?;

        delete(&tmp.path().join("iopole.db"), &InMemoryKeyStore::default())
    }

    #[test]
    fn creates_schema_on_first_open() -> Result<()> {
        let tmp = tempfile::tempdir()?;

        let connection = open(&tmp.path().join("iopole.db"), &InMemoryKeyStore::default())?;

        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        assert_eq!(usize::try_from(version).ok(), Some(MIGRATIONS.len()));
        connection
            .execute_batch("SELECT invoice_id FROM invoice; SELECT status FROM sync_range")?;
        Ok(())
    }

    #[test]
    fn migrations_are_applied_once() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        open(&path, &keys)?;

        open(&path, &keys)?;

        Ok(())
    }

    #[test]
    fn refuses_newer_schema() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        open(&path, &keys)?.pragma_update(None, "user_version", 99)?;

        let result = open(&path, &keys);

        assert!(matches!(result, Err(Error::UnsupportedSchema(99))));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn database_is_only_accessible_by_owner() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let tmp = tempfile::tempdir()?;
        let dir = tmp.path().join("iopole");
        let path = dir.join("iopole.db");

        open(&path, &InMemoryKeyStore::default())?;

        let mode =
            |p: &Path| -> Result<u32> { Ok(std::fs::metadata(p)?.permissions().mode() & 0o777) };
        assert_eq!(mode(&dir)?, 0o700);
        assert_eq!(mode(&path)?, 0o600);
        Ok(())
    }

    #[test]
    fn migration_extracts_identifier_values_of_any_scheme() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        let mut connection = Connection::open(&path)?;
        let key = generate_key()?;
        keys.save_key(&key)?;
        unlock(&connection, &key, &path)?;
        connection.execute_batch(MIGRATIONS[0])?;
        connection.execute_batch(
            "INSERT INTO invoice (invoice_id, direction, state, created_at, seller_address,
                              buyer_address, raw, synced_at)
         VALUES ('01a1', 'INBOUND', 'VALID', '', '0208:0123456789', '0225:352862346', '{}', ''),
                ('01a2', 'INBOUND', 'VALID', '', 'no-scheme', NULL, '{}', '');
         PRAGMA user_version = 1;",
        )?;

        migrate(&mut connection)?;

        let identifiers: Vec<(Option<String>, Option<String>)> = connection
            .prepare("SELECT seller_identifier, buyer_identifier FROM invoice ORDER BY invoice_id")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        assert_eq!(
            identifiers,
            [
                (Some("0123456789".to_owned()), Some("352862346".to_owned())),
                (None, None),
            ]
        );
        Ok(())
    }
}
