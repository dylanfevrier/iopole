pub(crate) mod search;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Show whether a company is registered with Iopole, and how
    Search(search::Args),
}
