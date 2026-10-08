use clap::{Args, Parser, Subcommand, ValueEnum};
use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;

use crate::invoice::Direction;

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
    /// Work with Iopole invoices
    Document {
        #[command(subcommand)]
        command: DocumentCommand,
    },
    /// Work with the business entities registered with Iopole
    Entity {
        #[command(subcommand)]
        command: EntityCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum EntityCommand {
    /// Show whether a company is registered with Iopole, and how
    Search(EntitySearchArgs),
}

#[derive(Debug, Args)]
pub struct EntitySearchArgs {
    /// Identifier value of the business entity (SIREN, SIRET, BCE...), without its scheme
    #[arg(value_parser = parse_identifier_value)]
    pub identifier_value: String,
}

#[derive(Debug, Subcommand)]
pub enum DocumentCommand {
    /// Synchronize Iopole invoices into the local database
    Sync(SyncArgs),
    /// Search synchronized invoices of a company by its electronic address
    Search(DocumentSearchArgs),
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

#[derive(Debug, Args)]
pub struct DocumentSearchArgs {
    /// Electronic address value of the seller or the buyer, whatever its scheme
    /// (352862346 matches 0225:352862346 and 0002:352862346)
    #[arg(value_parser = parse_identifier_value)]
    pub identifier_value: String,

    /// Only invoices where the company has this role
    #[arg(long, value_enum)]
    pub role: Option<RoleFilter>,

    /// Invoice flow
    #[arg(long, value_enum, default_value_t = DirectionFilter::Both)]
    pub direction: DirectionFilter,

    /// Iopole processing states, e.g. VALID,DELIVERED
    #[arg(long = "state", value_delimiter = ',', value_parser = parse_code)]
    pub states: Vec<String>,

    /// Last lifecycle statuses, e.g. REJECTED,APPROVED
    #[arg(long = "status", value_delimiter = ',', value_parser = parse_code)]
    pub statuses: Vec<String>,

    /// Created at or after: a UTC date (2026-09-01) or an RFC 3339 instant
    #[arg(long, value_parser = parse_instant)]
    pub from: Option<Timestamp>,

    /// Created before
    #[arg(long, value_parser = parse_instant)]
    pub to: Option<Timestamp>,

    /// Maximum number of invoices to show
    #[arg(long, default_value_t = 50)]
    pub limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DirectionFilter {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum RoleFilter {
    Seller,
    Buyer,
}

fn parse_identifier_value(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err("empty identifier".to_owned())
    } else if value.contains(':') {
        Err(format!(
            "`{value}` includes a scheme; pass only the value after `:`"
        ))
    } else {
        Ok(value.to_owned())
    }
}

fn parse_code(value: &str) -> Result<String, String> {
    let code = value.trim().to_uppercase();
    if code.is_empty() {
        Err("empty value".to_owned())
    } else {
        Ok(code)
    }
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
