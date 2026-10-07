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
mod tests;
