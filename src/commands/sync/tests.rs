use super::*;
use crate::clock::testing::FixedClock;
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
}

fn now() -> FixedClock {
    FixedClock(
        "2026-10-07T12:00:00Z"
            .parse()
            .unwrap_or_else(|err| panic!("{err}")),
    )
}

fn args(from: &str, to: &str, direction: DirectionFilter) -> Result<SyncArgs> {
    Ok(SyncArgs {
        from: from.parse()?,
        to: Some(to.parse()?),
        direction,
    })
}

fn sync(
    path: &Path,
    keys: &InMemoryKeyStore,
    args: &SyncArgs,
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

fn september_first(direction: DirectionFilter) -> Result<SyncArgs> {
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
fn reports_created_invoices_per_direction() -> Result<()> {
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
fn reports_only_the_requested_direction() -> Result<()> {
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
