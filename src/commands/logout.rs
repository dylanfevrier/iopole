use std::io::Write;
use std::path::Path;

use crate::Result;
use crate::credentials::CredentialStore;
use crate::database::{self, DatabaseKeyStore};

pub(crate) fn execute(
    credentials: &impl CredentialStore,
    database_path: &Path,
    database_keys: &impl DatabaseKeyStore,
    out: &mut impl Write,
) -> Result<()> {
    database::delete(database_path, database_keys)?;

    if !credentials.has_credentials()? {
        writeln!(out, "Not logged in.")?;
        return Ok(());
    }

    credentials.delete()?;
    writeln!(
        out,
        "Logged out. Credentials, local database and its encryption key removed."
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::testing::InMemoryStore;
    use crate::database::testing::InMemoryKeyStore;

    struct Fixture {
        _tmp: tempfile::TempDir,
        database_path: std::path::PathBuf,
        database_keys: InMemoryKeyStore,
    }

    impl Fixture {
        fn with_database() -> Result<Self> {
            let tmp = tempfile::tempdir()?;
            let database_path = tmp.path().join("iopole.db");
            let database_keys = InMemoryKeyStore::default();
            database::open(&database_path, &database_keys)?;
            Ok(Self {
                _tmp: tmp,
                database_path,
                database_keys,
            })
        }

        fn logout(&self, credentials: &InMemoryStore) -> Result<String> {
            let mut out = Vec::new();
            execute(
                credentials,
                &self.database_path,
                &self.database_keys,
                &mut out,
            )?;
            Ok(String::from_utf8_lossy(&out).into_owned())
        }
    }

    #[test]
    fn removes_credentials_database_and_key() -> Result<()> {
        let fixture = Fixture::with_database()?;
        let credentials = InMemoryStore::logged_in();

        let output = fixture.logout(&credentials)?;

        assert!(!credentials.has_credentials()?);
        assert!(!fixture.database_path.exists());
        assert!(fixture.database_keys.load_key()?.is_none());
        assert_eq!(
            output,
            "Logged out. Credentials, local database and its encryption key removed.\n"
        );
        Ok(())
    }

    #[test]
    fn removes_leftover_database_when_not_logged_in() -> Result<()> {
        let fixture = Fixture::with_database()?;

        let output = fixture.logout(&InMemoryStore::default())?;

        assert!(!fixture.database_path.exists());
        assert!(fixture.database_keys.load_key()?.is_none());
        assert_eq!(output, "Not logged in.\n");
        Ok(())
    }
}
