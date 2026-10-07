use super::*;

fn status(state: &str, last_date: Option<&str>) -> Status {
    Status {
        state: state.to_owned(),
        last_code: None,
        last_state: None,
        last_date: last_date.map(str::to_owned),
    }
}

fn party(address: &str) -> Party {
    Party {
        electronic_address: Some(address.to_owned()),
        ..Party::default()
    }
}

#[test]
fn reads_siren_from_0225_address() {
    assert_eq!(party("0225:352862346").siren(), Some("352862346"));
}

#[test]
fn ignores_other_address_schemes() {
    assert_eq!(party("0009:35286234600012").siren(), None);
    assert_eq!(party("0225:35286234").siren(), None);
    assert_eq!(party("0225:35286234A").siren(), None);
}

#[test]
fn newer_status_supersedes_older_one() {
    let stored = status("PENDING", Some("2026-10-07T19:37:02.672+00:00"));
    let incoming = status("DELIVERED", Some("2026-10-07T19:37:02.71+00:00"));

    assert!(incoming.supersedes(&stored));
    assert!(!stored.supersedes(&incoming));
}

#[test]
fn dates_are_compared_as_instants() {
    let stored = status("PENDING", Some("2026-10-07T21:00:00+02:00"));
    let incoming = status("DELIVERED", Some("2026-10-07T19:30:00Z"));

    assert!(incoming.supersedes(&stored));
}

#[test]
fn identical_status_is_not_rewritten() {
    let stored = status("VALID", Some("2026-10-07T19:37:02Z"));

    assert!(!stored.clone().supersedes(&stored));
}

#[test]
fn state_change_without_status_date_is_applied() {
    assert!(status("VALID", None).supersedes(&status("PENDING", None)));
}

#[test]
fn missing_status_never_erases_a_known_one() {
    let stored = status("VALID", Some("2026-10-07T19:37:02Z"));

    assert!(!status("PENDING", None).supersedes(&stored));
}
