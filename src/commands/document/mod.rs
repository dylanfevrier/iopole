pub(crate) mod search;
pub(crate) mod stat;
pub(crate) mod status;
pub(crate) mod sync;

use clap::{Subcommand, ValueEnum};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::invoice::Direction;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Synchronize Iopole invoices into the local database
    Sync(sync::Args),
    /// Search synchronized invoices, of every company or of one by its electronic address
    Search(search::Args),
    /// Count synchronized invoices by state, then by last status
    Stat(stat::Args),
    /// Show the status history of an invoice, live from Iopole
    Status(status::Args),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DirectionFilter {
    Inbound,
    Outbound,
    Both,
}

impl DirectionFilter {
    pub(crate) fn directions(self) -> &'static [Direction] {
        match self {
            Self::Inbound => &[Direction::Inbound],
            Self::Outbound => &[Direction::Outbound],
            Self::Both => &[Direction::Inbound, Direction::Outbound],
        }
    }
}

pub(crate) fn parse_instant(value: &str) -> std::result::Result<Timestamp, String> {
    value
        .parse::<Timestamp>()
        .or_else(|_| {
            value
                .parse::<Date>()
                .and_then(|date| date.to_zoned(TimeZone::UTC))
                .map(|midnight| midnight.timestamp())
        })
        .map_err(|_| format!("`{value}` is neither a date (YYYY-MM-DD) nor an RFC 3339 instant"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_are_utc_midnight() -> crate::Result<()> {
        assert_eq!(
            parse_instant("2026-09-01"),
            Ok("2026-09-01T00:00:00Z".parse()?)
        );
        Ok(())
    }

    #[test]
    fn instants_keep_their_offset() -> crate::Result<()> {
        assert_eq!(
            parse_instant("2026-09-01T02:00:00+02:00"),
            Ok("2026-09-01T00:00:00Z".parse()?)
        );
        Ok(())
    }

    #[test]
    fn rejects_other_formats() {
        assert!(parse_instant("01/09/2026").is_err());
    }
}
