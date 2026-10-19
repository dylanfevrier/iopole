use std::io::Write;
use std::path::Path;

use crate::Result;
use crate::credentials::CredentialStore;
use crate::database;

pub(crate) fn execute(
    credentials: &impl CredentialStore,
    database_path: &Path,
    out: &mut impl Write,
) -> Result<()> {
    database::delete(database_path)?;

    if !credentials.has_credentials()? {
        writeln!(out, "Not logged in.")?;
        return Ok(());
    }

    credentials.delete()?;
    writeln!(out, "Logged out. Credentials and local database removed.")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::testing::InMemoryStore;

    struct Fixture {
        _tmp: tempfile::TempDir,
        database_path: std::path::PathBuf,
    }

    impl Fixture {
        fn with_database() -> Result<Self> {
            let tmp = tempfile::tempdir()?;
            let database_path = tmp.path().join("iopole.db");
            database::open(&database_path)?;
            Ok(Self {
                _tmp: tmp,
                database_path,
            })
        }

        fn logout(&self, credentials: &InMemoryStore) -> Result<String> {
            let mut out = Vec::new();
            execute(credentials, &self.database_path, &mut out)?;
            Ok(String::from_utf8_lossy(&out).into_owned())
        }
    }

    #[test]
    fn removes_credentials_and_database() -> Result<()> {
        let fixture = Fixture::with_database()?;
        let credentials = InMemoryStore::logged_in();

        let output = fixture.logout(&credentials)?;

        assert!(!credentials.has_credentials()?);
        assert!(!fixture.database_path.exists());
        assert_eq!(
            output,
            "Logged out. Credentials and local database removed.\n"
        );
        Ok(())
    }

    #[test]
    fn removes_leftover_database_when_not_logged_in() -> Result<()> {
        let fixture = Fixture::with_database()?;

        let output = fixture.logout(&InMemoryStore::default())?;

        assert!(!fixture.database_path.exists());
        assert_eq!(output, "Not logged in.\n");
        Ok(())
    }
}
