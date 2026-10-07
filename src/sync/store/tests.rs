use super::*;
use crate::invoice::Party;
use crate::sync::testing::{TempStore, window};

fn invoice(id: &str, state: &str, last_date: Option<&str>) -> Invoice {
    Invoice {
        id: id.to_owned(),
        direction: "INBOUND".to_owned(),
        created_at: "2026-09-04T10:00:00Z".to_owned(),
        status: Status {
            state: state.to_owned(),
            last_code: None,
            last_state: None,
            last_date: last_date.map(str::to_owned),
        },
        number: Some("F-1".to_owned()),
        amount: Some("60".to_owned()),
        seller: Party::default(),
        buyer: Party::default(),
        raw: "{}".to_owned(),
    }
}

fn batch(window: TimeWindow, status: RangeStatus, invoices: &[Invoice]) -> WindowBatch<'_> {
    WindowBatch {
        direction: Direction::Inbound,
        window,
        status,
        invoices,
        synced_at: Timestamp::UNIX_EPOCH,
    }
}

fn day() -> TimeWindow {
    window("2026-09-04T00:00:00Z", "2026-09-05T00:00:00Z")
}

#[test]
fn inserts_new_invoices_and_records_range() -> Result<()> {
    let mut temp = TempStore::new()?;
    let invoices = [
        invoice("01a1", "VALID", None),
        invoice("01a2", "VALID", None),
    ];

    let stats = temp
        .store
        .commit(&batch(day(), RangeStatus::Complete, &invoices))?;

    assert_eq!(
        stats,
        WriteStats {
            inserted: 2,
            updated: 0
        }
    );
    assert_eq!(temp.invoice_ids()?, ["01a1", "01a2"]);
    assert_eq!(temp.store.covered(Direction::Inbound)?, [day()]);
    Ok(())
}

#[test]
fn replay_updates_only_newer_status() -> Result<()> {
    let mut temp = TempStore::new()?;
    let first = [invoice("01a1", "PENDING", Some("2026-09-04T10:00:00Z"))];
    temp.store
        .commit(&batch(day(), RangeStatus::Complete, &first))?;

    let newer = [invoice("01a1", "DELIVERED", Some("2026-09-04T11:00:00Z"))];
    let stats = temp
        .store
        .commit(&batch(day(), RangeStatus::Complete, &newer))?;
    let older = [invoice("01a1", "PENDING", Some("2026-09-04T09:00:00Z"))];
    let stale = temp
        .store
        .commit(&batch(day(), RangeStatus::Complete, &older))?;

    assert_eq!(
        stats,
        WriteStats {
            inserted: 0,
            updated: 1
        }
    );
    assert_eq!(stale, WriteStats::default());
    assert_eq!(temp.status_of("01a1")?, newer[0].status);
    Ok(())
}

#[test]
fn replay_never_rewrites_business_data() -> Result<()> {
    let mut temp = TempStore::new()?;
    temp.store.commit(&batch(
        day(),
        RangeStatus::Complete,
        &[invoice("01a1", "PENDING", None)],
    ))?;

    let changed = Invoice {
        raw: r#"{"changed":true}"#.to_owned(),
        ..invoice("01a1", "VALID", None)
    };
    temp.store
        .commit(&batch(day(), RangeStatus::Complete, &[changed]))?;

    assert_eq!(temp.raw_of("01a1")?, "{}");
    Ok(())
}

#[test]
fn open_and_truncated_ranges_are_not_covered() -> Result<()> {
    let mut temp = TempStore::new()?;
    let minute = window("2026-09-03T00:00:00Z", "2026-09-03T00:01:00Z");

    temp.store.commit(&batch(day(), RangeStatus::Open, &[]))?;
    temp.store
        .commit(&batch(minute, RangeStatus::Truncated, &[]))?;

    assert_eq!(temp.store.covered(Direction::Inbound)?, []);
    assert_eq!(temp.store.covered(Direction::Outbound)?, []);
    Ok(())
}

#[test]
fn completing_a_window_replaces_open_attempts_inside_it() -> Result<()> {
    let mut temp = TempStore::new()?;
    let morning = window("2026-09-04T00:00:00Z", "2026-09-04T12:00:00Z");
    temp.store.commit(&batch(morning, RangeStatus::Open, &[]))?;

    temp.store
        .commit(&batch(day(), RangeStatus::Complete, &[]))?;

    assert_eq!(
        temp.ranges()?,
        [(
            "2026-09-04T00:00:00Z".to_owned(),
            "2026-09-05T00:00:00Z".to_owned(),
            "complete".to_owned()
        )]
    );
    Ok(())
}

#[test]
fn failed_commit_leaves_nothing_behind() -> Result<()> {
    let mut temp = TempStore::new()?;
    // Invoices are written first, then the range fails: its start has no UUID v7 bound.
    let before_epoch = window("1969-12-31T00:00:00Z", "1970-01-02T00:00:00Z");
    let invoices = [invoice("01a1", "VALID", None)];

    let result = temp
        .store
        .commit(&batch(before_epoch, RangeStatus::Complete, &invoices));

    assert!(matches!(result, Err(crate::Error::InstantOutOfRange(_))));
    assert_eq!(temp.invoice_ids()?, Vec::<String>::new());
    assert_eq!(temp.ranges()?, []);
    Ok(())
}
