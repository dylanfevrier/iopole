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
    connection.execute_batch("SELECT invoice_id FROM invoice; SELECT status FROM sync_range")?;
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

    let mode = |p: &Path| -> Result<u32> { Ok(std::fs::metadata(p)?.permissions().mode() & 0o777) };
    assert_eq!(mode(&dir)?, 0o700);
    assert_eq!(mode(&path)?, 0o600);
    Ok(())
}
