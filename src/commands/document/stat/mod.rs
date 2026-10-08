mod render;

use std::io::Write;
use std::path::Path;

use clap::Args as ClapArgs;

use super::DirectionFilter;
use crate::Result;
use crate::catalog;
use crate::database::{self, DatabaseKeyStore};

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Invoice flow
    #[arg(long, value_enum, default_value_t = DirectionFilter::Both)]
    pub(crate) direction: DirectionFilter,
}

pub(crate) fn execute(
    database_path: &Path,
    keys: &impl DatabaseKeyStore,
    args: &Args,
    out: &mut impl Write,
) -> Result<()> {
    let counts = if database_path.exists() {
        catalog::count_by_state(
            &database::open(database_path, keys)?,
            args.direction.directions(),
        )?
    } else {
        Vec::new()
    };
    if counts.is_empty() {
        return render::no_invoice(out);
    }
    render::counts(out, counts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::testing::{InMemoryKeyStore, InvoiceRecord};

    fn stat(database_path: &Path, keys: &InMemoryKeyStore) -> Result<String> {
        stat_with(database_path, keys, DirectionFilter::Both)
    }

    fn stat_with(
        database_path: &Path,
        keys: &InMemoryKeyStore,
        direction: DirectionFilter,
    ) -> Result<String> {
        let mut out = Vec::new();
        execute(database_path, keys, &Args { direction }, &mut out)?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    #[test]
    fn counts_the_synchronized_invoices() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        let connection = database::open(&path, &keys)?;
        for (id, status) in [
            ("01a1", Some("RECEIVED")),
            ("01a2", Some("RECEIVED")),
            ("01a3", None),
        ] {
            InvoiceRecord {
                id,
                state: "DELIVERED",
                last_status_code: status,
                ..InvoiceRecord::default()
            }
            .insert(&connection)?;
        }

        assert_eq!(
            stat(&path, &keys)?,
            "\
STATE      STATUS       COUNT
DELIVERED  -                3
-          RECEIVED         2
-          (no status)      1

3 invoices.
"
        );
        Ok(())
    }

    #[test]
    fn counts_only_the_invoices_of_the_direction() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        let connection = database::open(&path, &keys)?;
        for (id, direction) in [
            ("01a1", "INBOUND"),
            ("01a2", "OUTBOUND"),
            ("01a3", "OUTBOUND"),
        ] {
            InvoiceRecord {
                id,
                direction,
                ..InvoiceRecord::default()
            }
            .insert(&connection)?;
        }

        assert!(stat_with(&path, &keys, DirectionFilter::Outbound)?.ends_with("\n2 invoices.\n"));
        assert!(stat_with(&path, &keys, DirectionFilter::Inbound)?.ends_with("\n1 invoices.\n"));
        assert!(stat(&path, &keys)?.ends_with("\n3 invoices.\n"));
        Ok(())
    }

    #[test]
    fn invites_to_sync_without_database() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();

        assert!(stat(&path, &keys)?.starts_with("No invoice synchronized yet.\n"));
        assert!(!path.exists());
        Ok(())
    }
}
