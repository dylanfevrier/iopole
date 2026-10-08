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
mod tests {
    use super::*;
    use crate::sync::testing::window;

    #[test]
    fn rejects_empty_windows() {
        let instant = Timestamp::UNIX_EPOCH;

        assert_eq!(TimeWindow::new(instant, instant), None);
    }

    #[test]
    fn splits_in_two_contiguous_halves() {
        let day = window("2026-09-04T00:00:00Z", "2026-09-05T00:00:00Z");

        let halves = day.split_half(SignedDuration::from_mins(1));

        assert_eq!(
            halves,
            Some((
                window("2026-09-04T00:00:00Z", "2026-09-04T12:00:00Z"),
                window("2026-09-04T12:00:00Z", "2026-09-05T00:00:00Z"),
            ))
        );
    }

    #[test]
    fn does_not_split_below_minimum() {
        let minute = window("2026-09-04T00:00:00Z", "2026-09-04T00:01:00Z");

        assert_eq!(minute.split_half(SignedDuration::from_mins(1)), None);
    }

    #[test]
    fn overlap_moves_start_back_by_percentage() {
        let day = window("2026-09-04T00:00:00Z", "2026-09-05T00:00:00Z");

        let replayed = day.with_overlap(10, Timestamp::UNIX_EPOCH);

        assert_eq!(
            replayed,
            window("2026-09-03T21:36:00Z", "2026-09-05T00:00:00Z")
        );
    }

    #[test]
    fn overlap_stops_at_floor() {
        let day = window("2026-09-04T00:00:00Z", "2026-09-05T00:00:00Z");

        let replayed = day.with_overlap(10, day.start());

        assert_eq!(replayed, day);
    }

    #[test]
    fn chunks_cover_window_exactly() {
        let span = window("2026-09-04T00:00:00Z", "2026-09-06T12:00:00Z");

        let chunks: Vec<_> = span.chunks(SignedDuration::from_hours(24)).collect();

        assert_eq!(
            chunks,
            vec![
                window("2026-09-04T00:00:00Z", "2026-09-05T00:00:00Z"),
                window("2026-09-05T00:00:00Z", "2026-09-06T00:00:00Z"),
                window("2026-09-06T00:00:00Z", "2026-09-06T12:00:00Z"),
            ]
        );
    }

    #[test]
    fn subtract_keeps_uncovered_parts() {
        let span = window("2026-09-01T00:00:00Z", "2026-09-10T00:00:00Z");
        let covered = window("2026-09-03T00:00:00Z", "2026-09-05T00:00:00Z");

        let rest: Vec<_> = span.subtract(covered).collect();

        assert_eq!(
            rest,
            vec![
                window("2026-09-01T00:00:00Z", "2026-09-03T00:00:00Z"),
                window("2026-09-05T00:00:00Z", "2026-09-10T00:00:00Z"),
            ]
        );
    }
}
