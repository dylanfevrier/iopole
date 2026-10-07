mod cli;
mod commands;
mod credentials;
mod error;

use std::io::Write;

pub use cli::{Cli, Command};
pub use error::{Error, Result};

use credentials::KeyringStore;

pub fn run(cli: &Cli, out: &mut impl Write) -> Result<()> {
    match cli.command {
        Command::Login => {
            commands::login::execute(&KeyringStore, commands::login::prompt_credentials, out)
        }
        Command::Logout => commands::logout::execute(&KeyringStore, out),
    }
}
