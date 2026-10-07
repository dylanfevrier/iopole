use std::io::Write;

use crate::Result;
use crate::credentials::CredentialStore;

pub(crate) fn execute(store: &impl CredentialStore, out: &mut impl Write) -> Result<()> {
    if !store.has_credentials()? {
        writeln!(out, "Not logged in.")?;
        return Ok(());
    }

    store.delete()?;
    writeln!(out, "Credentials removed from the system keyring.")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::testing::InMemoryStore;

    #[test]
    fn removes_stored_credentials() -> Result<()> {
        let store = InMemoryStore::logged_in();
        let mut out = Vec::new();

        execute(&store, &mut out)?;

        assert!(!store.has_credentials()?);
        assert_eq!(
            String::from_utf8_lossy(&out),
            "Credentials removed from the system keyring.\n"
        );
        Ok(())
    }

    #[test]
    fn reports_when_not_logged_in() -> Result<()> {
        let mut out = Vec::new();

        execute(&InMemoryStore::default(), &mut out)?;

        assert_eq!(String::from_utf8_lossy(&out), "Not logged in.\n");
        Ok(())
    }
}
