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
                seller_name, seller_address, seller_siren, seller_country,
                buyer_name, buyer_address, buyer_siren, buyer_country,
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
            invoice.seller.siren(),
            invoice.seller.country,
            invoice.buyer.name,
            invoice.buyer.electronic_address,
            invoice.buyer.siren(),
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
mod tests;
