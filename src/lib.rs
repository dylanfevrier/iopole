mod cli;
mod commands;
mod credentials;
mod database;
mod error;
mod keyring_store;

use std::io::Write;

pub use cli::{Cli, Command};
pub use error::{Error, Result};

use keyring_store::KeyringStore;

pub fn run(cli: &Cli, out: &mut impl Write) -> Result<()> {
    match cli.command {
        Command::Login => {
            commands::login::execute(&KeyringStore, commands::login::prompt_credentials, out)
        }
        Command::Logout => commands::logout::execute(
            &KeyringStore,
            &database::default_path()?,
            &KeyringStore,
            out,
        ),
        Command::Sync => commands::sync::execute(
            &KeyringStore,
            &database::default_path()?,
            &KeyringStore,
            out,
        ),
    }
}
