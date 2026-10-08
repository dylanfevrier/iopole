use jiff::Timestamp;
use rusqlite::{Connection, OptionalExtension, Transaction, params};

use super::window::TimeWindow;
use crate::Result;
use crate::invoice::{Direction, Invoice, Status};
use crate::iopole::UuidBound;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RangeStatus {
    Complete,
    Open,
    Truncated,
}

impl RangeStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Open => "open",
            Self::Truncated => "truncated",
        }
    }
}

pub(crate) struct WindowBatch<'a> {
    pub(crate) direction: Direction,
    pub(crate) window: TimeWindow,
    pub(crate) status: RangeStatus,
    pub(crate) invoices: &'a [Invoice],
    pub(crate) synced_at: Timestamp,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct WriteStats {
    pub(crate) inserted: usize,
    pub(crate) updated: usize,
}

pub(crate) trait InvoiceStore {
    fn covered(&self, direction: Direction) -> Result<Vec<TimeWindow>>;

    fn commit(&mut self, batch: &WindowBatch<'_>) -> Result<WriteStats>;
}

pub(crate) struct SqliteInvoiceStore {
    connection: Connection,
}

impl SqliteInvoiceStore {
    pub(crate) fn new(connection: Connection) -> Self {
        Self { connection }
    }
}

impl InvoiceStore for SqliteInvoiceStore {
    fn covered(&self, direction: Direction) -> Result<Vec<TimeWindow>> {
        let mut statement = self.connection.prepare(
            "SELECT from_at, to_at FROM sync_range WHERE direction = ?1 AND status = 'complete'",
        )?;
        let rows = statement.query_map([direction.as_str()], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut covered = Vec::new();
        for row in rows {
            let (from, to) = row?;
            covered.extend(TimeWindow::new(from.parse()?, to.parse()?));
        }
        Ok(covered)
    }

    fn commit(&mut self, batch: &WindowBatch<'_>) -> Result<WriteStats> {
        let transaction = self.connection.transaction()?;
        let synced_at = batch.synced_at.to_string();

        let mut stats = WriteStats::default();
        for invoice in batch.invoices {
            match stored_status(&transaction, &invoice.id)? {
                None => {
                    insert_invoice(&transaction, invoice, &synced_at)?;
                    stats.inserted += 1;
                }
                Some(stored) if invoice.status.supersedes(&stored) => {
                    update_status(&transaction, invoice, &synced_at)?;
                    stats.updated += 1;
                }
                Some(_) => {}
            }
        }
        record_range(&transaction, batch, &synced_at)?;

        transaction.commit()?;
        Ok(stats)
    }
}

fn stored_status(transaction: &Transaction<'_>, invoice_id: &str) -> Result<Option<Status>> {
    let status = transaction
        .prepare_cached(
            "SELECT state, last_status_code, last_status_state, last_status_date
             FROM invoice WHERE invoice_id = ?1",
        )?
        .query_row([invoice_id], |row| {
            Ok(Status {
                state: row.get(0)?,
                last_code: row.get(1)?,
                last_state: row.get(2)?,
                last_date: row.get(3)?,
            })
        })
        .optional()?;
    Ok(status)
}

fn insert_invoice(transaction: &Transaction<'_>, invoice: &Invoice, synced_at: &str) -> Result<()> {
    transaction
        .prepare_cached(
            "INSERT INTO invoice (
                invoice_id, direction, state, created_at,
                last_status_code, last_status_state, last_status_date,
                number, amount,
                seller_name, seller_address, seller_identifier, seller_country,
                buyer_name, buyer_address, buyer_identifier, buyer_country,
                raw, synced_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
        )?
        .execute(params![
            invoice.id,
            invoice.direction,
            invoice.status.state,
            invoice.created_at,
            invoice.status.last_code,
            invoice.status.last_state,
            invoice.status.last_date,
            invoice.number,
            invoice.amount,
            invoice.seller.name,
            invoice.seller.electronic_address,
            invoice.seller.identifier_value(),
            invoice.seller.country,
            invoice.buyer.name,
            invoice.buyer.electronic_address,
            invoice.buyer.identifier_value(),
            invoice.buyer.country,
            invoice.raw,
            synced_at,
        ])?;
    Ok(())
}

/// Business data stays as first synchronized.
fn update_status(transaction: &Transaction<'_>, invoice: &Invoice, synced_at: &str) -> Result<()> {
    transaction
        .prepare_cached(
            "UPDATE invoice SET
                state = ?2, last_status_code = ?3, last_status_state = ?4,
                last_status_date = ?5, synced_at = ?6
             WHERE invoice_id = ?1",
        )?
        .execute(params![
            invoice.id,
            invoice.status.state,
            invoice.status.last_code,
            invoice.status.last_state,
            invoice.status.last_date,
            synced_at,
        ])?;
    Ok(())
}

fn record_range(
    transaction: &Transaction<'_>,
    batch: &WindowBatch<'_>,
    synced_at: &str,
) -> Result<()> {
    let from_id = UuidBound::try_from(batch.window.start())?.to_string();
    let to_id = UuidBound::try_from(batch.window.end())?.to_string();
    let direction = batch.direction.as_str();

    transaction.execute(
        "DELETE FROM sync_range
         WHERE direction = ?1 AND status <> 'complete' AND from_id >= ?2 AND to_id <= ?3",
        params![direction, from_id, to_id],
    )?;
    transaction.execute(
        "INSERT INTO sync_range (direction, from_id, to_id, from_at, to_at, status, fetched, synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (direction, from_id, to_id) DO UPDATE SET
            status = excluded.status, fetched = excluded.fetched, synced_at = excluded.synced_at",
        params![
            direction,
            from_id,
            to_id,
            batch.window.start().to_string(),
            batch.window.end().to_string(),
            batch.status.as_str(),
            i64::try_from(batch.invoices.len()).unwrap_or(i64::MAX),
            synced_at,
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
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
}
