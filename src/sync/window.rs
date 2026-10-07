use std::fmt;

use jiff::{SignedDuration, Timestamp};

/// `[start, end)` in milliseconds, the resolution of UUID v7 timestamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TimeWindow {
    start: i64,
    end: i64,
}

impl TimeWindow {
    pub(crate) fn new(start: Timestamp, end: Timestamp) -> Option<Self> {
        Self::from_millis(start.as_millisecond(), end.as_millisecond())
    }

    fn from_millis(start: i64, end: i64) -> Option<Self> {
        (start < end).then_some(Self { start, end })
    }

    pub(crate) fn start(self) -> Timestamp {
        millis_to_timestamp(self.start)
    }

    pub(crate) fn end(self) -> Timestamp {
        millis_to_timestamp(self.end)
    }

    pub(crate) fn duration(self) -> SignedDuration {
        SignedDuration::from_millis(self.end - self.start)
    }

    pub(crate) fn split_half(self, min: SignedDuration) -> Option<(Self, Self)> {
        if self.duration() <= min {
            return None;
        }
        let middle = self.start + (self.end - self.start) / 2;
        Some((
            Self::from_millis(self.start, middle)?,
            Self::from_millis(middle, self.end)?,
        ))
    }

    pub(crate) fn with_overlap(self, percent: u8, floor: Timestamp) -> Self {
        let overlap = (self.end - self.start) * i64::from(percent) / 100;
        let start = (self.start - overlap).max(floor.as_millisecond().min(self.start));
        Self { start, ..self }
    }

    pub(crate) fn chunks(self, length: SignedDuration) -> impl Iterator<Item = Self> {
        let step = length.as_millis().max(1);
        let step = i64::try_from(step).unwrap_or(i64::MAX);
        std::iter::successors(Some(self.start), move |&start| {
            start.checked_add(step).filter(|&next| next < self.end)
        })
        .map(move |start| Self {
            start,
            end: start.saturating_add(step).min(self.end),
        })
    }

    pub(crate) fn subtract(self, other: Self) -> impl Iterator<Item = Self> {
        let before = Self::from_millis(self.start, other.start.min(self.end));
        let after = Self::from_millis(other.end.max(self.start), self.end);
        before.into_iter().chain(after)
    }

    pub(crate) fn ends_after(self, instant: Timestamp) -> bool {
        self.end > instant.as_millisecond()
    }
}

impl fmt::Display for TimeWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {})", self.start(), self.end())
    }
}

fn millis_to_timestamp(millis: i64) -> Timestamp {
    Timestamp::from_millisecond(millis).unwrap_or(Timestamp::MAX)
}

#[cfg(test)]
mod tests;
