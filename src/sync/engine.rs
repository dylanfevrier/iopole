use std::collections::HashSet;

use jiff::SignedDuration;

use super::coverage::gaps;
use super::store::{InvoiceStore, RangeStatus, WindowBatch};
use super::window::TimeWindow;
use super::{Progress, SyncConfig};
use crate::Result;
use crate::clock::Clock;
use crate::invoice::{Direction, Invoice};
use crate::iopole::{InvoiceSearch, SearchQuery, UuidBound};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SyncReport {
    pub(crate) complete_windows: usize,
    pub(crate) open_windows: usize,
    pub(crate) truncated_windows: usize,
    pub(crate) requests: usize,
    pub(crate) fetched: usize,
    pub(crate) inserted: usize,
    pub(crate) updated: usize,
}

enum Fetched {
    Complete(Vec<Invoice>),
    Truncated(Vec<Invoice>),
}

/// Each window `[a, b)` is searched as `[a - overlap, b)` but recorded as
/// `[a, b)`, so that no invoice falls between two searches.
pub(crate) struct Synchronizer<'a, S, St, C> {
    search: &'a mut S,
    store: &'a mut St,
    clock: &'a C,
    config: SyncConfig,
}

impl<'a, S: InvoiceSearch, St: InvoiceStore, C: Clock> Synchronizer<'a, S, St, C> {
    pub(crate) fn new(
        search: &'a mut S,
        store: &'a mut St,
        clock: &'a C,
        config: SyncConfig,
    ) -> Self {
        Self {
            search,
            store,
            clock,
            config,
        }
    }

    pub(crate) fn sync(
        &mut self,
        direction: Direction,
        requested: TimeWindow,
        progress: &mut impl Progress,
    ) -> Result<SyncReport> {
        let mut report = SyncReport::default();
        let gaps = gaps(requested, &self.store.covered(direction)?);
        let missing = gaps
            .iter()
            .fold(SignedDuration::ZERO, |total, gap| total + gap.duration());
        progress.advance(requested.duration() - missing);

        let mut pending: Vec<TimeWindow> = gaps
            .into_iter()
            .flat_map(|gap| gap.chunks(self.config.initial_window))
            .collect();
        pending.reverse();

        while let Some(window) = pending.pop() {
            let searched = window.with_overlap(self.config.overlap_percent, requested.start());
            let (status, invoices) = match self.fetch(direction, searched, &mut report)? {
                Fetched::Complete(invoices) => (self.status_of(window), invoices),
                Fetched::Truncated(invoices) => match window.split_half(self.config.min_window) {
                    Some((older, newer)) => {
                        pending.extend([newer, older]);
                        continue;
                    }
                    None => (RangeStatus::Truncated, invoices),
                },
            };
            self.commit(direction, window, status, &invoices, &mut report)?;
            progress.advance(window.duration());
        }
        Ok(report)
    }

    fn fetch(
        &mut self,
        direction: Direction,
        window: TimeWindow,
        report: &mut SyncReport,
    ) -> Result<Fetched> {
        let query = SearchQuery {
            direction,
            from: UuidBound::try_from(window.start())?,
            to: UuidBound::try_from(window.end())?,
        };
        let limit = self.config.page_limit;
        let mut invoices = Vec::new();
        let mut offset = 0;
        loop {
            let page = self.search.search(&query, offset, limit)?;
            report.requests += 1;
            let is_full = page.len() >= usize::try_from(limit).unwrap_or(usize::MAX);
            invoices.extend(page);

            if !is_full {
                return Ok(Fetched::Complete(deduplicate(invoices)));
            }
            offset += limit;
            if offset > self.config.max_offset {
                return Ok(Fetched::Truncated(deduplicate(invoices)));
            }
        }
    }

    fn status_of(&self, window: TimeWindow) -> RangeStatus {
        let settled_until = self.clock.now() - self.config.open_margin;
        if window.ends_after(settled_until) {
            RangeStatus::Open
        } else {
            RangeStatus::Complete
        }
    }

    fn commit(
        &mut self,
        direction: Direction,
        window: TimeWindow,
        status: RangeStatus,
        invoices: &[Invoice],
        report: &mut SyncReport,
    ) -> Result<()> {
        let written = self.store.commit(&WindowBatch {
            direction,
            window,
            status,
            invoices,
            synced_at: self.clock.now(),
        })?;
        report.fetched += invoices.len();
        report.inserted += written.inserted;
        report.updated += written.updated;
        match status {
            RangeStatus::Complete => report.complete_windows += 1,
            RangeStatus::Open => report.open_windows += 1,
            RangeStatus::Truncated => report.truncated_windows += 1,
        }
        Ok(())
    }
}

/// Results are sorted most recent first: invoices arriving while pages are
/// read shift them.
fn deduplicate(mut invoices: Vec<Invoice>) -> Vec<Invoice> {
    let mut seen = HashSet::new();
    invoices.retain(|invoice| seen.insert(invoice.id.clone()));
    invoices
}

#[cfg(test)]
mod tests;
