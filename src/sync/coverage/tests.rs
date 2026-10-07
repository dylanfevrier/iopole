use super::*;
use crate::sync::testing::window;

fn requested() -> TimeWindow {
    window("2026-09-01T00:00:00Z", "2026-09-10T00:00:00Z")
}

#[test]
fn everything_is_a_gap_without_coverage() {
    assert_eq!(gaps(requested(), &[]), vec![requested()]);
}

#[test]
fn no_gap_when_fully_covered() {
    let covered = [window("2026-08-01T00:00:00Z", "2026-10-01T00:00:00Z")];

    assert_eq!(gaps(requested(), &covered), vec![]);
}

#[test]
fn adjacent_windows_leave_no_gap() {
    let covered = [
        window("2026-09-05T00:00:00Z", "2026-09-10T00:00:00Z"),
        window("2026-09-01T00:00:00Z", "2026-09-05T00:00:00Z"),
    ];

    assert_eq!(gaps(requested(), &covered), vec![]);
}

#[test]
fn finds_every_hole_in_order() {
    let covered = [
        window("2026-09-06T00:00:00Z", "2026-09-07T00:00:00Z"),
        window("2026-09-02T00:00:00Z", "2026-09-04T00:00:00Z"),
        window("2026-09-03T00:00:00Z", "2026-09-05T00:00:00Z"),
    ];

    assert_eq!(
        gaps(requested(), &covered),
        vec![
            window("2026-09-01T00:00:00Z", "2026-09-02T00:00:00Z"),
            window("2026-09-05T00:00:00Z", "2026-09-06T00:00:00Z"),
            window("2026-09-07T00:00:00Z", "2026-09-10T00:00:00Z"),
        ]
    );
}
