mod coverage;
mod engine;
mod store;
mod window;

#[cfg(test)]
mod testing;

pub(crate) use engine::Synchronizer;
pub(crate) use store::SqliteInvoiceStore;
pub(crate) use window::TimeWindow;

use jiff::SignedDuration;

use crate::iopole::{MAX_OFFSET, MAX_PAGE_LIMIT};

#[derive(Debug, Clone, Copy)]
pub(crate) struct SyncConfig {
    pub(crate) initial_window: SignedDuration,
    pub(crate) min_window: SignedDuration,
    pub(crate) overlap_percent: u8,
    pub(crate) open_margin: SignedDuration,
    pub(crate) page_limit: u32,
    pub(crate) max_offset: u32,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            initial_window: SignedDuration::from_hours(24),
            min_window: SignedDuration::from_mins(1),
            overlap_percent: 10,
            open_margin: SignedDuration::from_hours(1),
            page_limit: MAX_PAGE_LIMIT,
            max_offset: MAX_OFFSET,
        }
    }
}

pub(crate) trait Progress {
    /// Receives the length of each part of the requested period once synchronized.
    fn advance(&mut self, synchronized: SignedDuration);
}
