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
    /// Value of the electronic address, whatever its scheme: `352862346` for
    /// `0225:352862346`, `0123456789` for `0208:0123456789`.
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
mod tests;
