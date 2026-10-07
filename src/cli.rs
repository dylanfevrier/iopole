//! Définition de l'interface en ligne de commande.

use clap::Parser;

/// Outil en ligne de commande pour interagir avec la plateforme Iopole.
#[derive(Debug, Parser)]
#[command(name = "iopole", version, about, long_about = None)]
pub struct Cli {}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
