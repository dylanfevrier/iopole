mod args;
mod render;

use std::io::Write;
use std::path::Path;

pub(crate) use args::Args;

use crate::catalog::{self, InvoiceParties};
use crate::credentials::{CredentialStore, Credentials};
use crate::database::{self, DatabaseKeyStore};
use crate::iopole::InvoiceStatuses;
use crate::{Error, Result};

pub(crate) fn execute<S: InvoiceStatuses>(
    credentials: &impl CredentialStore,
    database_path: &Path,
    keys: &impl DatabaseKeyStore,
    connect: impl FnOnce(Credentials) -> S,
    args: &Args,
    out: &mut impl Write,
) -> Result<()> {
    let credentials = credentials.load()?.ok_or(Error::NotLoggedIn)?;
    let mut statuses = connect(credentials);
    if args.json {
        return match statuses.raw_status_history(&args.invoice_id)? {
            Some(history) => render::json(out, &history),
            None => render::unknown_invoice(out, &args.invoice_id),
        };
    }
    let Some(history) = statuses.status_history(&args.invoice_id)? else {
        return render::unknown_invoice(out, &args.invoice_id);
    };
    let synchronized = synchronized_parties(database_path, keys, &args.invoice_id)?;
    render::history(out, &args.invoice_id, synchronized.as_ref(), history)
}

/// Statuses name the parties generically: their real names come from the
/// invoice, when it is synchronized.
fn synchronized_parties(
    database_path: &Path,
    keys: &impl DatabaseKeyStore,
    invoice_id: &str,
) -> Result<Option<InvoiceParties>> {
    if !database_path.exists() {
        return Ok(None);
    }
    catalog::parties(&database::open(database_path, keys)?, invoice_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::credentials::testing::InMemoryStore;
    use crate::database::testing::{InMemoryKeyStore, InvoiceRecord};
    use crate::iopole::testing::{FakeStatuses, payment_received};

    const INVOICE_ID: &str = "01a10b05-e3fd-74c8-b9cd-ce8e4c012777";

    fn args() -> Args {
        Args {
            invoice_id: INVOICE_ID.to_owned(),
            json: false,
        }
    }

    fn run(
        database_path: &Path,
        keys: &InMemoryKeyStore,
        statuses: FakeStatuses,
    ) -> Result<String> {
        run_with(database_path, keys, statuses, &args())
    }

    fn run_with(
        database_path: &Path,
        keys: &InMemoryKeyStore,
        statuses: FakeStatuses,
        args: &Args,
    ) -> Result<String> {
        let mut out = Vec::new();
        execute(
            &InMemoryStore::logged_in(),
            database_path,
            keys,
            |_| statuses,
            args,
            &mut out,
        )?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    fn known(history: Vec<crate::iopole::StatusEvent>) -> FakeStatuses {
        FakeStatuses {
            history: Some(history),
            ..FakeStatuses::default()
        }
    }

    #[test]
    fn prints_the_history_as_iopole_returns_it_with_json() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        let raw = serde_json::json!([{ "date": "2026-10-05T09:45:07.000Z" }]);

        let output = run_with(
            &path,
            &keys,
            FakeStatuses {
                raw: Some(raw),
                ..FakeStatuses::default()
            },
            &Args {
                json: true,
                ..args()
            },
        )?;

        assert_eq!(
            output,
            "[\n  {\n    \"date\": \"2026-10-05T09:45:07.000Z\"\n  }\n]\n"
        );
        assert!(!path.exists());
        Ok(())
    }

    #[test]
    fn names_the_parties_of_a_synchronized_invoice() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        InvoiceRecord {
            id: INVOICE_ID,
            seller_name: Some("Leroy Merlin France"),
            buyer_name: Some("BVS RENOVATION"),
            ..InvoiceRecord::default()
        }
        .insert(&database::open(&path, &keys)?)?;

        let output = run(&path, &keys, known(vec![payment_received()]))?;

        assert!(output.contains("Seller:       Leroy Merlin France (SIREN 384560942)\n"));
        assert!(output.contains("Buyer:        BVS RENOVATION (SIREN 980217632)\n"));
        Ok(())
    }

    #[test]
    fn works_without_local_database() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();

        let output = run(&path, &keys, known(vec![payment_received()]))?;

        assert!(output.contains("Seller:       VENDEUR (SIREN 384560942)\n"));
        assert!(!path.exists());
        assert!(keys.load_key()?.is_none());
        Ok(())
    }

    #[test]
    fn reports_an_invoice_unknown_to_iopole() -> Result<()> {
        let tmp = tempfile::tempdir()?;

        let output = run(
            &tmp.path().join("iopole.db"),
            &InMemoryKeyStore::default(),
            FakeStatuses::default(),
        )?;

        assert_eq!(output, format!("Iopole knows no invoice {INVOICE_ID}.\n"));
        Ok(())
    }

    #[test]
    fn refuses_when_not_logged_in() -> Result<()> {
        let tmp = tempfile::tempdir()?;

        let result = execute(
            &InMemoryStore::default(),
            &tmp.path().join("iopole.db"),
            &InMemoryKeyStore::default(),
            |_| -> FakeStatuses { panic!("must not connect without credentials") },
            &args(),
            &mut Vec::new(),
        );

        assert!(matches!(result, Err(Error::NotLoggedIn)));
        Ok(())
    }
}
