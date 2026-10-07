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
mod tests;
