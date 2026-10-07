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
