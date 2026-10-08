use jiff::Timestamp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Inbound,
    Outbound,
}

impl Direction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Inbound => "INBOUND",
            Self::Outbound => "OUTBOUND",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Invoice {
    /// Iopole identifier, not the issuer's invoice number.
    pub(crate) id: String,
    pub(crate) direction: String,
    pub(crate) created_at: String,
    pub(crate) status: Status,
    pub(crate) number: Option<String>,
    pub(crate) amount: Option<String>,
    pub(crate) seller: Party,
    pub(crate) buyer: Party,
    pub(crate) raw: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Party {
    pub(crate) name: Option<String>,
    pub(crate) electronic_address: Option<String>,
    pub(crate) country: Option<String>,
}

impl Party {
    pub(crate) fn identifier_value(&self) -> Option<&str> {
        let (_, value) = self.electronic_address.as_deref()?.split_once(':')?;
        (!value.is_empty()).then_some(value)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Status {
    pub(crate) state: String,
    pub(crate) last_code: Option<String>,
    pub(crate) last_state: Option<String>,
    pub(crate) last_date: Option<String>,
}

impl Status {
    /// A missing or unreadable date counts as older than any date.
    pub(crate) fn supersedes(&self, stored: &Self) -> bool {
        self != stored
            && parse_date(self.last_date.as_deref()) >= parse_date(stored.last_date.as_deref())
    }
}

fn parse_date(date: Option<&str>) -> Option<Timestamp> {
    date?.parse().ok()
}

#[cfg(test)]
mod tests {
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
}
