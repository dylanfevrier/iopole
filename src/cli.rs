use clap::{Parser, Subcommand};

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
    /// Synchronize Iopole data into the local database
    Sync,
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
