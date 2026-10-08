use std::fmt::Write as _;

use clap::{Args as ClapArgs, ValueEnum};
use jiff::Timestamp;

use super::super::{DirectionFilter, parse_instant};
use crate::catalog::{InvoiceFilter, Role};
use crate::cli::parse_identifier_value;
use crate::iopole::UuidBound;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Electronic address value of the seller or the buyer, whatever its scheme
    /// (352862346 matches 0225:352862346 and 0002:352862346)
    #[arg(value_parser = parse_identifier_value)]
    pub(crate) identifier_value: String,

    /// Only invoices where the company has this role
    #[arg(long, value_enum)]
    pub(crate) role: Option<RoleFilter>,

    /// Invoice flow
    #[arg(long, value_enum, default_value_t = DirectionFilter::Both)]
    pub(crate) direction: DirectionFilter,

    /// Iopole processing states, e.g. VALID,DELIVERED
    #[arg(long = "state", value_delimiter = ',', value_parser = parse_code)]
    pub(crate) states: Vec<String>,

    /// Last lifecycle statuses, e.g. REJECTED,APPROVED
    #[arg(long = "status", value_delimiter = ',', value_parser = parse_code)]
    pub(crate) statuses: Vec<String>,

    /// Created at or after: a UTC date (2026-09-01) or an RFC 3339 instant
    #[arg(long, value_parser = parse_instant)]
    pub(crate) from: Option<Timestamp>,

    /// Created before
    #[arg(long, value_parser = parse_instant)]
    pub(crate) to: Option<Timestamp>,

    /// Maximum number of invoices to show
    #[arg(long, default_value_t = 50)]
    pub(crate) limit: usize,
}

impl Args {
    pub(crate) fn filter(&self) -> crate::Result<InvoiceFilter> {
        Ok(InvoiceFilter {
            identifier_value: self.identifier_value.clone(),
            role: self.role.map(|role| match role {
                RoleFilter::Seller => Role::Seller,
                RoleFilter::Buyer => Role::Buyer,
            }),
            directions: self.direction.directions().to_vec(),
            states: self.states.clone(),
            statuses: self.statuses.clone(),
            from: self.from.map(UuidBound::try_from).transpose()?,
            to: self.to.map(UuidBound::try_from).transpose()?,
            limit: self.limit,
        })
    }

    /// The `document sync` command retrieving what this search looks for.
    pub(crate) fn sync_command(&self) -> String {
        let mut command = match self.from {
            Some(from) => format!("iopole document sync --from {from}"),
            None => "iopole document sync --from <date>".to_owned(),
        };
        if let Some(to) = self.to {
            let _ = write!(command, " --to {to}");
        }
        match self.direction {
            DirectionFilter::Inbound => command.push_str(" --direction inbound"),
            DirectionFilter::Outbound => command.push_str(" --direction outbound"),
            DirectionFilter::Both => {}
        }
        command
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum RoleFilter {
    Seller,
    Buyer,
}

fn parse_code(value: &str) -> Result<String, String> {
    let code = value.trim().to_uppercase();
    if code.is_empty() {
        Err("empty value".to_owned())
    } else {
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parse_args;

    #[test]
    fn accepts_several_states_in_any_case() -> Result<(), clap::Error> {
        let args = parse_args::<Args>(&[
            "352862346",
            "--state",
            "valid,Delivered",
            "--state",
            "pending",
        ])?;

        assert_eq!(args.states, ["VALID", "DELIVERED", "PENDING"]);
        assert_eq!(args.role, None);
        assert_eq!(args.limit, 50);
        Ok(())
    }

    #[test]
    fn takes_identifier_values_without_their_scheme() -> Result<(), clap::Error> {
        assert_eq!(
            parse_args::<Args>(&["0123456789"])?.identifier_value,
            "0123456789"
        );
        assert!(parse_args::<Args>(&["0225:352862346"]).is_err());
        assert!(parse_args::<Args>(&[" "]).is_err());
        Ok(())
    }

    #[test]
    fn suggests_a_sync_of_the_searched_period_and_direction() -> Result<(), clap::Error> {
        let args = parse_args::<Args>(&[
            "352862346",
            "--from",
            "2026-09-01",
            "--to",
            "2026-10-01",
            "--direction",
            "inbound",
        ])?;

        assert_eq!(
            args.sync_command(),
            "iopole document sync --from 2026-09-01T00:00:00Z --to 2026-10-01T00:00:00Z --direction inbound"
        );
        assert_eq!(
            parse_args::<Args>(&["352862346"])?.sync_command(),
            "iopole document sync --from <date>"
        );
        Ok(())
    }
}
