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
mod tests {
    use super::*;

    fn bound(instant: &str) -> Result<String> {
        let instant: Timestamp = instant.parse()?;
        Ok(UuidBound::try_from(instant)?.to_string())
    }

    #[test]
    fn matches_documented_vectors() -> Result<()> {
        assert_eq!(
            bound("2026-09-04T00:00:00Z")?,
            "01a069b7-1000-0000-0000-000000000000"
        );
        assert_eq!(
            bound("2026-09-05T00:00:00Z")?,
            "01a06edd-6c00-0000-0000-000000000000"
        );
        Ok(())
    }

    #[test]
    fn real_invoice_id_falls_within_its_day() -> Result<()> {
        let id = "01a06bdb-d3a5-70c6-9838-6bec1a1fef9f";

        assert!(bound("2026-09-04T00:00:00Z")?.as_str() <= id);
        assert!(id < bound("2026-09-05T00:00:00Z")?.as_str());
        Ok(())
    }

    #[test]
    fn rejects_instants_before_epoch() -> Result<()> {
        let instant: Timestamp = "1969-12-31T23:59:59Z".parse()?;

        assert!(matches!(
            UuidBound::try_from(instant),
            Err(Error::InstantOutOfRange(_))
        ));
        Ok(())
    }
}
