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
    /// Remove Iopole credentials from the system keyring
    Logout,
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
