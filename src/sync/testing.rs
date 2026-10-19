use std::path::PathBuf;

use jiff::{SignedDuration, Timestamp};
use rusqlite::Connection;

use super::Progress;
use super::store::SqliteInvoiceStore;
use super::window::TimeWindow;
use crate::Result;
use crate::database;
use crate::invoice::Status;

pub(crate) fn window(start: &str, end: &str) -> TimeWindow {
    let parse = |s: &str| {
        s.parse::<Timestamp>()
            .unwrap_or_else(|err| panic!("{s}: {err}"))
    };
    TimeWindow::new(parse(start), parse(end)).unwrap_or_else(|| panic!("empty window"))
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RecordedProgress(pub(crate) SignedDuration);

impl Progress for RecordedProgress {
    fn advance(&mut self, synchronized: SignedDuration) {
        self.0 += synchronized;
    }
}

pub(crate) struct TempStore {
    pub(crate) store: SqliteInvoiceStore,
    path: PathBuf,
    _tmp: tempfile::TempDir,
}

impl TempStore {
    pub(crate) fn new() -> Result<Self> {
        let tmp = tempfile::tempdir()?;
        let path = tmp.path().join("iopole.db");
        let store = SqliteInvoiceStore::new(database::open(&path)?);
        Ok(Self {
            store,
            path,
            _tmp: tmp,
        })
    }

    fn connection(&self) -> Result<Connection> {
        database::open(&self.path)
    }

    pub(crate) fn invoice_ids(&self) -> Result<Vec<String>> {
        let connection = self.connection()?;
        let mut statement =
            connection.prepare("SELECT invoice_id FROM invoice ORDER BY invoice_id")?;
        let ids = statement
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(ids)
    }

    pub(crate) fn status_of(&self, invoice_id: &str) -> Result<Status> {
        Ok(self.connection()?.query_row(
            "SELECT state, last_status_code, last_status_state, last_status_date
             FROM invoice WHERE invoice_id = ?1",
            [invoice_id],
            |row| {
                Ok(Status {
                    state: row.get(0)?,
                    last_code: row.get(1)?,
                    last_state: row.get(2)?,
                    last_date: row.get(3)?,
                })
            },
        )?)
    }

    pub(crate) fn ranges(&self) -> Result<Vec<(String, String, String)>> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT from_at, to_at, status FROM sync_range ORDER BY from_id, to_id")?;
        let ranges = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(ranges)
    }

    pub(crate) fn raw_of(&self, invoice_id: &str) -> Result<String> {
        Ok(self.connection()?.query_row(
            "SELECT raw FROM invoice WHERE invoice_id = ?1",
            [invoice_id],
            |row| row.get(0),
        )?)
    }
}
