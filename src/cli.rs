use clap::{Parser, Subcommand};

use crate::commands::{auth, directory, document, entity};

#[derive(Debug, Parser)]
#[command(name = "iopole", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Log in to Iopole or log out
    Auth {
        #[command(subcommand)]
        command: auth::Command,
    },
    /// Work with Iopole invoices
    Document {
        #[command(subcommand)]
        command: document::Command,
    },
    /// Work with the business entities registered with Iopole
    Entity {
        #[command(subcommand)]
        command: entity::Command,
    },
    /// Look up companies in the public e-invoicing directories
    Directory {
        #[command(subcommand)]
        command: directory::Command,
    },
}

pub(crate) fn parse_identifier_value(value: &str) -> Result<String, String> {
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

/// Parses the arguments of a single command, without the commands around it.
#[cfg(test)]
pub(crate) fn parse_args<A: clap::Args>(args: &[&str]) -> Result<A, clap::Error> {
    #[derive(Parser)]
    struct Only<A: clap::Args> {
        #[command(flatten)]
        args: A,
    }
    Ok(Only::<A>::try_parse_from(["iopole"].iter().chain(args))?.args)
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
