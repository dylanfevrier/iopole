use std::io::Write;
use std::path::Path;

use crate::credentials::CredentialStore;
use crate::database::{self, DatabaseKeyStore};
use crate::{Error, Result};

pub(crate) fn execute(
    credentials: &impl CredentialStore,
    database_path: &Path,
    keys: &impl DatabaseKeyStore,
    out: &mut impl Write,
) -> Result<()> {
    if !credentials.has_credentials()? {
        return Err(Error::NotLoggedIn);
    }

    let is_new = !database_path.exists();
    database::open(database_path, keys)?;

    if is_new {
        writeln!(out, "Database created at {}", database_path.display())?;
    } else {
        writeln!(out, "Using database at {}", database_path.display())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::testing::InMemoryStore;
    use crate::database::testing::InMemoryKeyStore;

    fn output_of_sync(path: &Path, keys: &InMemoryKeyStore) -> Result<String> {
        let mut out = Vec::new();
        execute(&InMemoryStore::logged_in(), path, keys, &mut out)?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    #[test]
    fn creates_database_on_first_sync() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");

        let output = output_of_sync(&path, &InMemoryKeyStore::default())?;

        assert!(path.is_file());
        assert_eq!(output, format!("Database created at {}\n", path.display()));
        Ok(())
    }

    #[test]
    fn reuses_existing_database() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        output_of_sync(&path, &keys)?;

        let output = output_of_sync(&path, &keys)?;

        assert_eq!(output, format!("Using database at {}\n", path.display()));
        Ok(())
    }

    #[test]
    fn refuses_to_sync_when_not_logged_in() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");

        let result = execute(
            &InMemoryStore::default(),
            &path,
            &InMemoryKeyStore::default(),
            &mut Vec::new(),
        );

        assert!(matches!(result, Err(Error::NotLoggedIn)));
        assert!(!path.exists());
        Ok(())
    }
}
