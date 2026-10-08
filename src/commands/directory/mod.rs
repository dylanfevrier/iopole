pub(crate) mod french;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Search the French directory (annuaire) by SIREN, SIRET or name
    French(french::Args),
}
