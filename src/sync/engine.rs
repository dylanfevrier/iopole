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
mod tests {
    use jiff::{SignedDuration, Timestamp};

    use super::*;
    use crate::clock::testing::FixedClock;
    use crate::iopole::testing::{FakeSearch, id_at, inbound, invoice};
    use crate::sync::testing::{RecordedProgress, TempStore, window};

    fn small_pages() -> SyncConfig {
        // At most 4 results per search: offsets 0 and 2, 2 results each.
        SyncConfig {
            page_limit: 2,
            max_offset: 2,
            ..SyncConfig::default()
        }
    }

    fn later() -> FixedClock {
        FixedClock(
            "2026-10-07T00:00:00Z"
                .parse()
                .unwrap_or_else(|err| panic!("{err}")),
        )
    }

    fn run(
        search: &mut FakeSearch,
        temp: &mut TempStore,
        config: SyncConfig,
        requested: TimeWindow,
    ) -> Result<SyncReport> {
        run_at(search, temp, config, requested, &later())
    }

    fn run_at(
        search: &mut FakeSearch,
        temp: &mut TempStore,
        config: SyncConfig,
        requested: TimeWindow,
        clock: &FixedClock,
    ) -> Result<SyncReport> {
        let mut progress = RecordedProgress::default();
        let report = Synchronizer::new(search, &mut temp.store, clock, config).sync(
            Direction::Inbound,
            requested,
            &mut progress,
        )?;
        assert_eq!(progress.0, requested.duration(), "progress must reach 100%");
        Ok(report)
    }

    #[test]
    fn stores_every_invoice_including_on_window_boundaries() -> Result<()> {
        let mut temp = TempStore::new()?;
        let invoices = vec![
            inbound("2026-09-01T00:00:00Z", 1),
            inbound("2026-09-01T23:59:59.999Z", 2),
            inbound("2026-09-02T00:00:00Z", 3),
            inbound("2026-09-03T12:00:00Z", 4),
            inbound("2026-09-03T23:59:59.999Z", 5),
        ];
        let mut search = FakeSearch::with(invoices.clone());

        let report = run(
            &mut search,
            &mut temp,
            SyncConfig::default(),
            window("2026-09-01T00:00:00Z", "2026-09-04T00:00:00Z"),
        )?;

        let mut expected: Vec<_> = invoices.into_iter().map(|invoice| invoice.id).collect();
        expected.sort();
        assert_eq!(temp.invoice_ids()?, expected);
        assert_eq!(report.complete_windows, 3);
        assert_eq!(report.inserted, 5);
        Ok(())
    }

    #[test]
    fn each_search_replays_ten_percent_of_the_previous_window() -> Result<()> {
        let mut temp = TempStore::new()?;
        let mut search = FakeSearch::default();

        run(
            &mut search,
            &mut temp,
            SyncConfig::default(),
            window("2026-09-01T00:00:00Z", "2026-09-03T00:00:00Z"),
        )?;

        let searched: Vec<_> = search
            .queries
            .iter()
            .map(|(query, _)| (query.from, query.to))
            .collect();
        let bound = |instant: &str| UuidBound::try_from(instant.parse::<Timestamp>()?);
        assert_eq!(
            searched,
            [
                // The first window does not go before the requested period.
                (
                    bound("2026-09-01T00:00:00Z")?,
                    bound("2026-09-02T00:00:00Z")?
                ),
                (
                    bound("2026-09-01T21:36:00Z")?,
                    bound("2026-09-03T00:00:00Z")?
                ),
            ]
        );
        Ok(())
    }

    #[test]
    fn invoices_found_again_by_the_overlap_are_inserted_once() -> Result<()> {
        let mut temp = TempStore::new()?;
        let mut search = FakeSearch::with(vec![inbound("2026-09-01T23:00:00Z", 1)]);

        let report = run(
            &mut search,
            &mut temp,
            SyncConfig::default(),
            window("2026-09-01T00:00:00Z", "2026-09-03T00:00:00Z"),
        )?;

        // Read by the first window, then by the overlap of the second one.
        assert_eq!((report.fetched, report.inserted, report.updated), (2, 1, 0));
        assert_eq!(temp.invoice_ids()?.len(), 1);
        Ok(())
    }

    #[test]
    fn overlap_refreshes_status_of_invoices_near_the_gap() -> Result<()> {
        let mut temp = TempStore::new()?;
        let id = id_at("2026-09-01T23:00:00Z", 1);
        let mut search = FakeSearch::with(vec![invoice(
            id.clone(),
            "PENDING",
            Some("2026-09-01T23:00:00Z"),
        )]);
        run(
            &mut search,
            &mut temp,
            SyncConfig::default(),
            window("2026-09-01T00:00:00Z", "2026-09-02T00:00:00Z"),
        )?;

        search.invoices = vec![invoice(
            id.clone(),
            "DELIVERED",
            Some("2026-09-02T08:00:00Z"),
        )];
        let report = run(
            &mut search,
            &mut temp,
            SyncConfig::default(),
            window("2026-09-01T00:00:00Z", "2026-09-03T00:00:00Z"),
        )?;

        assert_eq!((report.inserted, report.updated), (0, 1));
        assert_eq!(temp.status_of(&id)?.state, "DELIVERED");
        Ok(())
    }

    #[test]
    fn second_sync_of_a_covered_period_makes_no_call() -> Result<()> {
        let mut temp = TempStore::new()?;
        let mut search = FakeSearch::with(vec![inbound("2026-09-01T12:00:00Z", 1)]);
        let requested = window("2026-09-01T00:00:00Z", "2026-09-05T00:00:00Z");
        run(&mut search, &mut temp, SyncConfig::default(), requested)?;
        search.queries.clear();

        let report = run(&mut search, &mut temp, SyncConfig::default(), requested)?;

        assert_eq!(report, SyncReport::default());
        assert_eq!(search.queries, []);
        Ok(())
    }

    #[test]
    fn truncated_windows_are_split_until_complete() -> Result<()> {
        let mut temp = TempStore::new()?;
        let mut invoices: Vec<_> = (1..=6)
            .map(|n| inbound("2026-09-01T03:00:00Z", n))
            .collect();
        invoices.push(inbound("2026-09-01T18:00:00Z", 7));
        let mut search = FakeSearch::with(invoices);

        let report = run(
            &mut search,
            &mut temp,
            small_pages(),
            window("2026-09-01T00:00:00Z", "2026-09-02T00:00:00Z"),
        )?;

        // 6 invoices on the same millisecond cannot be split: only 4 are returned.
        assert_eq!(report.truncated_windows, 1);
        assert_eq!(temp.invoice_ids()?.len(), 4 + 1);
        assert!(report.complete_windows > 1);
        Ok(())
    }

    #[test]
    fn dense_minute_is_recorded_as_truncated_and_searched_again() -> Result<()> {
        let mut temp = TempStore::new()?;
        let invoices: Vec<_> = (1..=5)
            .map(|n| inbound("2026-09-01T00:00:30Z", n))
            .collect();
        let mut search = FakeSearch::with(invoices);
        let minute = window("2026-09-01T00:00:00Z", "2026-09-01T00:01:00Z");

        let report = run(&mut search, &mut temp, small_pages(), minute)?;
        search.queries.clear();
        run(&mut search, &mut temp, small_pages(), minute)?;

        assert_eq!(report.truncated_windows, 1);
        assert_eq!(report.inserted, 4, "what was read is kept");
        assert!(
            !search.queries.is_empty(),
            "a truncated window is not covered"
        );
        Ok(())
    }

    #[test]
    fn recent_windows_stay_open_and_are_searched_again() -> Result<()> {
        let mut temp = TempStore::new()?;
        let mut search = FakeSearch::default();
        let today = window("2026-10-06T00:00:00Z", "2026-10-06T23:30:00Z");
        let now = FixedClock("2026-10-07T00:00:00Z".parse()?);

        let report = run_at(&mut search, &mut temp, SyncConfig::default(), today, &now)?;
        search.queries.clear();
        let an_hour_later = FixedClock(now.0 + SignedDuration::from_hours(1));
        let again = run_at(
            &mut search,
            &mut temp,
            SyncConfig::default(),
            today,
            &an_hour_later,
        )?;

        assert_eq!(report.open_windows, 1);
        assert_eq!(again.complete_windows, 1);
        assert_eq!(search.queries.len(), 1);
        assert_eq!(
            temp.ranges()?,
            [(
                "2026-10-06T00:00:00Z".to_owned(),
                "2026-10-06T23:30:00Z".to_owned(),
                "complete".to_owned()
            )]
        );
        Ok(())
    }

    #[test]
    fn only_gaps_are_searched() -> Result<()> {
        let mut temp = TempStore::new()?;
        let mut search = FakeSearch::default();
        run(
            &mut search,
            &mut temp,
            SyncConfig::default(),
            window("2026-09-02T00:00:00Z", "2026-09-03T00:00:00Z"),
        )?;
        search.queries.clear();

        let report = run(
            &mut search,
            &mut temp,
            SyncConfig::default(),
            window("2026-09-01T00:00:00Z", "2026-09-04T00:00:00Z"),
        )?;

        assert_eq!(report.complete_windows, 2);
        assert_eq!(search.queries.len(), 2);
        Ok(())
    }

    #[test]
    fn duplicated_results_across_pages_are_stored_once() {
        let first = inbound("2026-09-01T00:00:00Z", 1);
        let second = inbound("2026-09-01T00:00:01Z", 2);

        let unique = deduplicate(vec![first.clone(), second.clone(), first.clone()]);

        assert_eq!(unique, [first, second]);
    }
}
