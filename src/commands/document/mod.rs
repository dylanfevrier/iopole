pub(crate) mod search;
pub(crate) mod sync;

use clap::{Subcommand, ValueEnum};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;

use crate::invoice::Direction;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Synchronize Iopole invoices into the local database
    Sync(sync::Args),
    /// Search synchronized invoices of a company by its electronic address
    Search(search::Args),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
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

fn parse_instant(value: &str) -> std::result::Result<Timestamp, String> {
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
