mod args;
mod render;

use std::io::Write;
use std::path::Path;

pub(crate) use args::Args;

use crate::clock::Clock;
use crate::credentials::{CredentialStore, Credentials};
use crate::database::{self, DatabaseKeyStore};
use crate::iopole::InvoiceSearch;
use crate::sync::{SqliteInvoiceStore, SyncConfig, Synchronizer};
use crate::{Error, Result};
use render::ProgressBar;

pub(crate) fn execute<S: InvoiceSearch>(
    credentials: &impl CredentialStore,
    database_path: &Path,
    keys: &impl DatabaseKeyStore,
    connect: impl FnOnce(Credentials) -> S,
    clock: &impl Clock,
    args: &Args,
    out: &mut impl Write,
) -> Result<()> {
    let period = args.period(clock)?;
    let credentials = credentials.load()?.ok_or(Error::NotLoggedIn)?;
    let directions = args.direction.directions();

    let mut store = SqliteInvoiceStore::new(database::open(database_path, keys)?);
    let mut search = connect(credentials);
    let mut synchronizer = Synchronizer::new(&mut search, &mut store, clock, SyncConfig::default());
    let mut progress = ProgressBar::new(directions.iter().map(|_| period.duration()).sum());

    let mut retrieved = Vec::with_capacity(directions.len());
    let mut updated = 0;
    let mut truncated_windows = 0;
    for &direction in directions {
        let report = synchronizer.sync(direction, period, args.mode(), &mut progress)?;
        retrieved.push((direction, report.inserted));
        updated += report.updated;
        truncated_windows += report.truncated_windows;
    }
    progress.finish();

    let updated = args.refresh.then_some(updated);
    render::report(out, &retrieved, updated, truncated_windows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::testing::FixedClock;
    use crate::commands::document::DirectionFilter;
    use crate::credentials::testing::InMemoryStore;
    use crate::database::testing::InMemoryKeyStore;
    use crate::invoice::Invoice;
    use crate::iopole::testing::{FakeSearch, inbound, outbound};

    struct UnreachableStore;

    impl CredentialStore for UnreachableStore {
        fn has_credentials(&self) -> Result<bool> {
            unreachable!("credentials must not be read")
        }

        fn load(&self) -> Result<Option<Credentials>> {
            unreachable!("credentials must not be read")
        }

        fn save(&self, _: &Credentials) -> Result<()> {
            unreachable!("credentials must not be written")
        }

        fn delete(&self) -> Result<()> {
            unreachable!("credentials must not be deleted")
        }

        fn location(&self) -> String {
            unreachable!("credentials must not be located")
        }
    }

    fn now() -> FixedClock {
        FixedClock(
            "2026-10-07T12:00:00Z"
                .parse()
                .unwrap_or_else(|err| panic!("{err}")),
        )
    }

    fn args(from: &str, to: &str, direction: DirectionFilter) -> Result<Args> {
        Ok(Args {
            from: from.parse()?,
            to: Some(to.parse()?),
            direction,
            refresh: false,
        })
    }

    fn sync(
        path: &Path,
        keys: &InMemoryKeyStore,
        args: &Args,
        invoices: Vec<Invoice>,
    ) -> Result<String> {
        let mut out = Vec::new();
        execute(
            &InMemoryStore::logged_in(),
            path,
            keys,
            |_| FakeSearch::with(invoices),
            &now(),
            args,
            &mut out,
        )?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    fn september_first(direction: DirectionFilter) -> Result<Args> {
        args("2026-09-01T00:00:00Z", "2026-09-02T00:00:00Z", direction)
    }

    fn invoices() -> Vec<Invoice> {
        vec![
            inbound("2026-09-01T08:00:00Z", 1),
            inbound("2026-09-01T09:00:00Z", 2),
            outbound("2026-09-01T10:00:00Z", 3),
        ]
    }

    #[test]
    fn stores_the_invoices_of_the_period_in_the_database() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");

        let output = sync(
            &path,
            &InMemoryKeyStore::default(),
            &september_first(DirectionFilter::Both)?,
            invoices(),
        )?;

        assert!(path.is_file());
        assert_eq!(output, "3 invoices retrieved. 2 inbound. 1 outbound.\n");
        Ok(())
    }

    #[test]
    fn syncs_only_the_requested_direction() -> Result<()> {
        let tmp = tempfile::tempdir()?;

        let output = sync(
            &tmp.path().join("iopole.db"),
            &InMemoryKeyStore::default(),
            &september_first(DirectionFilter::Outbound)?,
            invoices(),
        )?;

        assert_eq!(output, "1 invoices retrieved. 1 outbound.\n");
        Ok(())
    }

    #[test]
    fn second_sync_creates_nothing() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        let both = september_first(DirectionFilter::Both)?;
        sync(&path, &keys, &both, invoices())?;

        let output = sync(&path, &keys, &both, invoices())?;

        assert_eq!(output, "0 invoices retrieved. 0 inbound. 0 outbound.\n");
        Ok(())
    }

    #[test]
    fn refresh_rewrites_what_changed_and_tells_how_many() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let keys = InMemoryKeyStore::default();
        sync(
            &path,
            &keys,
            &september_first(DirectionFilter::Both)?,
            invoices(),
        )?;
        let mut changed = invoices();
        changed[0].amount = Some("72".to_owned());
        let refresh = Args {
            refresh: true,
            ..september_first(DirectionFilter::Both)?
        };

        let output = sync(&path, &keys, &refresh, changed)?;

        assert_eq!(
            output,
            "0 invoices retrieved. 0 inbound. 0 outbound.\n1 invoices updated.\n"
        );
        Ok(())
    }

    #[test]
    fn refuses_empty_period_before_reading_credentials_or_database() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let reversed = args(
            "2026-09-02T00:00:00Z",
            "2026-09-01T00:00:00Z",
            DirectionFilter::Both,
        )?;

        let result = execute(
            &UnreachableStore,
            &path,
            &InMemoryKeyStore::default(),
            |_| FakeSearch::default(),
            &now(),
            &reversed,
            &mut Vec::new(),
        );

        assert!(matches!(result, Err(Error::EmptyPeriod { .. })));
        assert!(!path.exists());
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
            |_| FakeSearch::default(),
            &now(),
            &september_first(DirectionFilter::Inbound)?,
            &mut Vec::new(),
        );

        assert!(matches!(result, Err(Error::NotLoggedIn)));
        assert!(!path.exists());
        Ok(())
    }
}
