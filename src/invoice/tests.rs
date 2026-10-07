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
fn identifier_value_ignores_the_scheme() {
    assert_eq!(
        party("0225:352862346").identifier_value(),
        Some("352862346")
    );
    assert_eq!(
        party("0002:352862346").identifier_value(),
        Some("352862346")
    );
    assert_eq!(
        party("0208:0123456789").identifier_value(),
        Some("0123456789")
    );
}

#[test]
fn address_without_value_has_no_identifier() {
    assert_eq!(party("352862346").identifier_value(), None);
    assert_eq!(party("0225:").identifier_value(), None);
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
