pub(crate) mod search;
pub(crate) mod update_claim;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Show whether a company is registered with Iopole, and how
    Search(search::Args),
    /// Replace how the operator claims a business entity: its company UID and invoice flow
    UpdateClaim(update_claim::Args),
}
