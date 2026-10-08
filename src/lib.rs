mod catalog;
mod cli;
mod clock;
mod commands;
mod credentials;
mod database;
mod error;
mod invoice;
mod iopole;
mod keyring_store;
mod render;
mod sync;

use std::io::Write;

pub use cli::Cli;
pub use error::{Error, Result};

use cli::Command;
use clock::SystemClock;
use commands::{auth, directory, document, entity};
use iopole::{Endpoints, HttpClient};
use keyring_store::KeyringStore;

pub fn run(cli: &Cli, out: &mut impl Write) -> Result<()> {
    let keyring = KeyringStore::default();
    match &cli.command {
        Command::Auth {
            command: auth::Command::Login,
        } => auth::login::execute(&keyring, auth::login::prompt_credentials, out),
        Command::Auth {
            command: auth::Command::Logout,
        } => auth::logout::execute(&keyring, &database::default_path()?, &keyring, out),
        Command::Document {
            command: document::Command::Sync(args),
        } => commands::document::sync::execute(
            &keyring,
            &database::default_path()?,
            &keyring,
            |credentials| HttpClient::new(Endpoints::from_env(), credentials),
            &SystemClock,
            args,
            out,
        ),
        Command::Document {
            command: document::Command::Search(args),
        } => commands::document::search::execute(&database::default_path()?, &keyring, args, out),
        Command::Document {
            command: document::Command::Status(args),
        } => commands::document::status::execute(
            &keyring,
            &database::default_path()?,
            &keyring,
            |credentials| HttpClient::new(Endpoints::from_env(), credentials),
            args,
            out,
        ),
        Command::Entity {
            command: entity::Command::Search(args),
        } => commands::entity::search::execute(
            &keyring,
            |credentials| HttpClient::new(Endpoints::from_env(), credentials),
            args,
            out,
        ),
        Command::Directory {
            command: directory::Command::French(args),
        } => commands::directory::french::execute(
            &keyring,
            |credentials| HttpClient::new(Endpoints::from_env(), credentials),
            args,
            out,
        ),
    }
}
