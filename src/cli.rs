use clap::{Args, Parser, Subcommand, ValueEnum};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;

#[derive(Debug, Parser)]
#[command(name = "iopole", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Store Iopole credentials in the system keyring
    Login,
    /// Log out and remove credentials, local database and its encryption key
    Logout,
    /// Synchronize Iopole invoices into the local database
    Sync(SyncArgs),
}

#[derive(Debug, Args)]
pub struct SyncArgs {
    /// Start of the period, included: a UTC date (2026-09-01) or an RFC 3339 instant
    #[arg(long, value_parser = parse_instant)]
    pub from: Timestamp,

    /// End of the period, excluded [default: now]
    #[arg(long, value_parser = parse_instant)]
    pub to: Option<Timestamp>,

    /// Invoice flow to synchronize
    #[arg(long, value_enum, default_value_t = DirectionFilter::Both)]
    pub direction: DirectionFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DirectionFilter {
    Inbound,
    Outbound,
    Both,
}

fn parse_instant(value: &str) -> Result<Timestamp, String> {
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
mod tests;
