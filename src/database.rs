use std::fs::{DirBuilder, OpenOptions};
use std::path::{Path, PathBuf};

use etcetera::BaseStrategy;
use rusqlite::{Connection, ErrorCode};

use crate::{Error, Result};

const APP_DIR: &str = "iopole";
const DATABASE_FILE: &str = "iopole.db";
const SQLITE_SIDE_FILE_SUFFIXES: [&str; 3] = ["-journal", "-wal", "-shm"];

const MIGRATIONS: [&str; 2] = [
    include_str!("migrations/001_invoices.sql"),
    include_str!("migrations/002_party_identifiers.sql"),
];

pub(crate) fn default_path() -> Result<PathBuf> {
    let data_dir = etcetera::choose_base_strategy()?.data_dir();
    Ok(data_dir.join(APP_DIR).join(DATABASE_FILE))
}

pub(crate) fn open(path: &Path) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        create_private_dir(dir)?;
    }
    create_private_file(path)?;

    let mut connection = Connection::open(path)?;
    check_readable(&connection, path)?;
    migrate(&mut connection)?;
    Ok(connection)
}

pub(crate) fn delete(path: &Path) -> Result<()> {
    remove_if_exists(path)?;
    for suffix in SQLITE_SIDE_FILE_SUFFIXES {
        let mut side_file = path.as_os_str().to_owned();
        side_file.push(suffix);
        remove_if_exists(Path::new(&side_file))?;
    }
    Ok(())
}

fn remove_if_exists(path: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(path) {
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

/// Databases written by earlier versions are encrypted, and unreadable now.
fn check_readable(connection: &Connection, path: &Path) -> Result<()> {
    match connection.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(())) {
        Ok(()) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(err, _)) if err.code == ErrorCode::NotADatabase => {
            Err(Error::UnreadableDatabase(path.to_owned()))
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
    use crate::Result;

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
    use super::*;

    fn create_populated_database(path: &Path) -> Result<()> {
        open(path)?
            .execute_batch("CREATE TABLE probe (id INTEGER); INSERT INTO probe VALUES (1)")?;
        Ok(())
    }

    #[test]
    fn creates_database_and_missing_directories() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("nested/iopole.db");

        create_populated_database(&path)?;

        assert!(path.is_file());
        Ok(())
    }

    #[test]
    fn database_file_is_plain_sqlite() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");

        create_populated_database(&path)?;

        let header = std::fs::read(&path)?;
        assert!(header.starts_with(b"SQLite format 3"));
        Ok(())
    }

    #[test]
    fn reopening_reads_existing_data() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        create_populated_database(&path)?;

        let count: i64 =
            open(&path)?.query_row("SELECT count(*) FROM probe", [], |row| row.get(0))?;

        assert_eq!(count, 1);
        Ok(())
    }

    #[test]
    fn refuses_unreadable_database() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        std::fs::write(&path, [0x5a_u8; 4096])?;

        let result = open(&path);

        assert!(matches!(result, Err(Error::UnreadableDatabase(_))));
        Ok(())
    }

    #[test]
    fn delete_removes_database() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        create_populated_database(&path)?;

        delete(&path)?;

        assert!(!path.exists());
        Ok(())
    }

    #[test]
    fn delete_succeeds_when_database_does_not_exist() -> Result<()> {
        let tmp = tempfile::tempdir()?;

        delete(&tmp.path().join("iopole.db"))
    }

    #[test]
    fn creates_schema_on_first_open() -> Result<()> {
        let tmp = tempfile::tempdir()?;

        let connection = open(&tmp.path().join("iopole.db"))?;

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
        open(&path)?;

        open(&path)?;

        Ok(())
    }

    #[test]
    fn refuses_newer_schema() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        open(&path)?.pragma_update(None, "user_version", 99)?;

        let result = open(&path);

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

        open(&path)?;

        let mode =
            |p: &Path| -> Result<u32> { Ok(std::fs::metadata(p)?.permissions().mode() & 0o777) };
        assert_eq!(mode(&dir)?, 0o700);
        assert_eq!(mode(&path)?, 0o600);
        Ok(())
    }

    #[test]
    fn migration_extracts_identifier_values_of_any_scheme() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let mut connection = Connection::open(tmp.path().join("iopole.db"))?;
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
