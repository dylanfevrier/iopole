use std::io::Write;
use std::path::Path;

use indicatif::ProgressStyle;
use jiff::SignedDuration;

use crate::cli::{DirectionFilter, SyncArgs};
use crate::clock::Clock;
use crate::credentials::{CredentialStore, Credentials};
use crate::database::{self, DatabaseKeyStore};
use crate::invoice::Direction;
use crate::iopole::{InvoiceSearch, UuidBound};
use crate::sync::{Progress, SqliteInvoiceStore, SyncConfig, Synchronizer, TimeWindow};
use crate::{Error, Result};

pub(crate) fn execute<S: InvoiceSearch>(
    credentials: &impl CredentialStore,
    database_path: &Path,
    keys: &impl DatabaseKeyStore,
    connect: impl FnOnce(Credentials) -> S,
    clock: &impl Clock,
    args: &SyncArgs,
    out: &mut impl Write,
) -> Result<()> {
    let period = requested_period(args, clock)?;
    let credentials = credentials.load()?.ok_or(Error::NotLoggedIn)?;
    let directions = directions(args.direction);

    let mut store = SqliteInvoiceStore::new(database::open(database_path, keys)?);
    let mut search = connect(credentials);
    let mut synchronizer = Synchronizer::new(&mut search, &mut store, clock, SyncConfig::default());
    let mut progress = ProgressBar::new(directions.iter().map(|_| period.duration()).sum());

    let mut retrieved = Vec::with_capacity(directions.len());
    let mut truncated_windows = 0;
    for &direction in directions {
        let report = synchronizer.sync(direction, period, &mut progress)?;
        retrieved.push((direction, report.inserted));
        truncated_windows += report.truncated_windows;
    }
    progress.finish();

    let total: usize = retrieved.iter().map(|(_, inserted)| inserted).sum();
    write!(out, "{total} invoices retrieved.")?;
    for (direction, inserted) in retrieved {
        write!(out, " {inserted} {}.", direction.as_str().to_lowercase())?;
    }
    writeln!(out)?;
    if truncated_windows > 0 {
        writeln!(
            out,
            "warning: {truncated_windows} one-minute windows hold more invoices than Iopole returns and may be incomplete"
        )?;
    }
    Ok(())
}

fn requested_period(args: &SyncArgs, clock: &impl Clock) -> Result<TimeWindow> {
    let (from, to) = (args.from, args.to.unwrap_or_else(|| clock.now()));
    UuidBound::try_from(from)?;
    UuidBound::try_from(to)?;
    TimeWindow::new(from, to).ok_or(Error::EmptyPeriod { from, to })
}

fn directions(filter: DirectionFilter) -> &'static [Direction] {
    match filter {
        DirectionFilter::Inbound => &[Direction::Inbound],
        DirectionFilter::Outbound => &[Direction::Outbound],
        DirectionFilter::Both => &[Direction::Inbound, Direction::Outbound],
    }
}

struct ProgressBar(indicatif::ProgressBar);

impl ProgressBar {
    fn new(total: SignedDuration) -> Self {
        let style = ProgressStyle::with_template("{bar:40} {percent:>3}%")
            .unwrap_or_else(|_| ProgressStyle::default_bar());
        Self(indicatif::ProgressBar::new(millis(total)).with_style(style))
    }

    fn finish(&self) {
        self.0.finish();
    }
}

impl Progress for ProgressBar {
    fn advance(&mut self, synchronized: SignedDuration) {
        self.0.inc(millis(synchronized));
    }
}

fn millis(duration: SignedDuration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(0)
}

#[cfg(test)]
mod tests;
