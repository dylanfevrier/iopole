use clap::{Args as ClapArgs, ValueEnum};

use crate::cli::parse_identifier_value;
use crate::invoice::Direction;
use crate::iopole::Claim;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Identifier value of the business entity (SIREN, SIRET, BCE...), without its scheme
    #[arg(value_parser = parse_identifier_value)]
    pub(crate) identifier_value: String,

    /// Identifier of the company in our system, sent in the `X-Company-UID` webhook header
    #[arg(long, alias = "companyUid", value_parser = parse_company_uid)]
    pub(crate) company_uid: String,

    /// Invoice flow the claim covers
    #[arg(long, value_enum, default_value_t = ClaimDirection::Both)]
    pub(crate) direction: ClaimDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ClaimDirection {
    Inbound,
    Outbound,
    Both,
}

impl ClaimDirection {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
            Self::Both => "both",
        }
    }
}

impl Args {
    pub(crate) fn claim(&self) -> Claim {
        Claim {
            company_uid: self.company_uid.clone(),
            direction: match self.direction {
                ClaimDirection::Inbound => Some(Direction::Inbound),
                ClaimDirection::Outbound => Some(Direction::Outbound),
                ClaimDirection::Both => None,
            },
        }
    }
}

fn parse_company_uid(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err("empty company UID".to_owned())
    } else {
        Ok(value.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parse_args;

    const SIREN: &str = "980217632";

    #[test]
    fn covers_both_directions_by_default() -> Result<(), clap::Error> {
        let args = parse_args::<Args>(&[SIREN, "--company-uid", "6071797"])?;

        assert_eq!(
            args.claim(),
            Claim {
                company_uid: "6071797".to_owned(),
                direction: None,
            }
        );
        Ok(())
    }

    #[test]
    fn takes_a_direction_and_the_camel_case_spelling() -> Result<(), clap::Error> {
        let args =
            parse_args::<Args>(&[SIREN, "--companyUid", "6071797", "--direction", "outbound"])?;

        assert_eq!(args.claim().direction, Some(Direction::Outbound));
        Ok(())
    }

    #[test]
    fn requires_a_company_uid() {
        assert!(parse_args::<Args>(&[SIREN]).is_err());
        assert!(parse_args::<Args>(&[SIREN, "--company-uid", " "]).is_err());
    }

    #[test]
    fn takes_an_identifier_value_without_its_scheme() {
        assert!(parse_args::<Args>(&["0225:980217632", "--company-uid", "6071797"]).is_err());
    }
}
