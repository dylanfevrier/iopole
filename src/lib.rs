mod cli;
mod clock;
mod commands;
mod credentials;
mod database;
mod error;
mod invoice;
mod iopole;
mod keyring_store;
mod sync;

use std::io::Write;

pub use cli::{Cli, Command, DirectionFilter, SyncArgs};
pub use error::{Error, Result};

use clock::SystemClock;
use iopole::{Endpoints, HttpClient};
use keyring_store::KeyringStore;

pub fn run(cli: &Cli, out: &mut impl Write) -> Result<()> {
    let keyring = KeyringStore::default();
    match &cli.command {
        Command::Login => {
            commands::login::execute(&keyring, commands::login::prompt_credentials, out)
        }
        Command::Logout => {
            commands::logout::execute(&keyring, &database::default_path()?, &keyring, out)
        }
        Command::Sync(args) => commands::sync::execute(
            &keyring,
            &database::default_path()?,
            &keyring,
            |credentials| HttpClient::new(Endpoints::from_env(), credentials),
            &SystemClock,
            args,
            out,
        ),
    }
}
