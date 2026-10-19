use clap::{Args as ClapArgs, ValueEnum};

use crate::cli::parse_identifier_value;
use crate::iopole::Network;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Identifier value of the business entity (SIREN, SIRET, BCE...), without its scheme
    #[arg(value_parser = parse_identifier_value)]
    pub(crate) identifier_value: String,

    /// Network to publish the identifier on
    #[arg(long, value_enum, default_value_t = NetworkChoice::Both)]
    pub(crate) network: NetworkChoice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum NetworkChoice {
    DomesticFr,
    PeppolInternational,
    Both,
}

impl NetworkChoice {
    /// The French directory first, as einvoicing registers them.
    pub(crate) fn networks(self) -> &'static [Network] {
        match self {
            Self::DomesticFr => &[Network::DomesticFr],
            Self::PeppolInternational => &[Network::PeppolInternational],
            Self::Both => &[Network::DomesticFr, Network::PeppolInternational],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parse_args;

    const SIREN: &str = "982740128";

    #[test]
    fn covers_both_networks_by_default() -> Result<(), clap::Error> {
        let args = parse_args::<Args>(&[SIREN])?;

        assert_eq!(
            args.network.networks(),
            [Network::DomesticFr, Network::PeppolInternational]
        );
        Ok(())
    }

    #[test]
    fn takes_a_single_network() -> Result<(), clap::Error> {
        let args = parse_args::<Args>(&[SIREN, "--network", "peppol-international"])?;

        assert_eq!(args.network.networks(), [Network::PeppolInternational]);
        Ok(())
    }

    #[test]
    fn takes_an_identifier_value_without_its_scheme() {
        assert!(parse_args::<Args>(&["0002:982740128"]).is_err());
    }
}
