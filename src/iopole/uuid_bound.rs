use std::fmt;

use jiff::Timestamp;

use crate::{Error, Result};

const MAX_MILLIS: i64 = (1 << 48) - 1;

/// Iopole invoice identifiers are UUID v7: their first 48 bits are the creation
/// time in milliseconds, so `[bound(t1), bound(t2))` selects every invoice
/// created in `[t1, t2)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct UuidBound(i64);

impl TryFrom<Timestamp> for UuidBound {
    type Error = Error;

    fn try_from(instant: Timestamp) -> Result<Self> {
        let millis = instant.as_millisecond();
        if (0..=MAX_MILLIS).contains(&millis) {
            Ok(Self(millis))
        } else {
            Err(Error::InstantOutOfRange(instant))
        }
    }
}

impl fmt::Display for UuidBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex = format!("{:012x}", self.0);
        write!(f, "{}-{}-0000-0000-000000000000", &hex[..8], &hex[8..])
    }
}

#[cfg(test)]
mod tests;
