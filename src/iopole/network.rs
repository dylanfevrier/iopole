/// Networks on which Iopole publishes an identifier, so that other platforms
/// route invoices to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Network {
    /// The French directory (annuaire).
    DomesticFr,
    PeppolInternational,
}

impl Network {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::DomesticFr => "DOMESTIC_FR",
            Self::PeppolInternational => "PEPPOL_INTERNATIONAL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Registration {
    Registered,
    /// The directory already publishes the identifier: Iopole changed nothing.
    AlreadyUpToDate,
}

/// Iopole refuses a registration that changes nothing with a 404 and, for
/// `DOMESTIC_FR`, the code `DOMESTIC_FR_REGISTER_DIRECTORY_ALREADY_UP_TO_DATE`.
pub(crate) fn is_already_up_to_date(code: &str) -> bool {
    code.ends_with("_ALREADY_UP_TO_DATE")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_a_registration_that_changes_nothing() {
        assert!(is_already_up_to_date(
            "DOMESTIC_FR_REGISTER_DIRECTORY_ALREADY_UP_TO_DATE"
        ));
        assert!(!is_already_up_to_date("BUSINESS_ENTITY_NOT_FOUND"));
    }
}
