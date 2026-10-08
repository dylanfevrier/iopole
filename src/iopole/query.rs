use std::fmt;

use super::uuid_bound::UuidBound;
use crate::invoice::Direction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SearchQuery {
    pub(crate) direction: Direction,
    pub(crate) from: UuidBound,
    pub(crate) to: UuidBound,
}

impl fmt::Display for SearchQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"invoice.direction:"{}" AND invoice.invoiceId:>="{}" AND invoice.invoiceId:<"{}""#,
            self.direction.as_str(),
            self.from,
            self.to,
        )
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;

    use super::*;
    use crate::Result;

    fn bound(instant: &str) -> Result<UuidBound> {
        UuidBound::try_from(instant.parse::<Timestamp>()?)
    }

    #[test]
    fn renders_documented_query() -> Result<()> {
        let query = SearchQuery {
            direction: Direction::Inbound,
            from: bound("2026-09-04T00:00:00Z")?,
            to: bound("2026-09-05T00:00:00Z")?,
        };

        assert_eq!(
            query.to_string(),
            r#"invoice.direction:"INBOUND" AND invoice.invoiceId:>="01a069b7-1000-0000-0000-000000000000" AND invoice.invoiceId:<"01a06edd-6c00-0000-0000-000000000000""#
        );
        Ok(())
    }
}
