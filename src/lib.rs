mod catalog;
mod cli;
mod clock;
mod commands;
mod credentials;
mod database;
mod error;
mod invoice;
mod iopole;
mod render;
mod secrets_file;
mod sync;

use std::io::Write;

pub use cli::Cli;
pub use error::{Error, Result};

use cli::Command;
use clock::SystemClock;
use commands::{auth, directory, document, entity};
use iopole::{Endpoints, HttpClient};
use secrets_file::SecretsFile;

pub fn run(cli: &Cli, out: &mut impl Write) -> Result<()> {
    let secrets = SecretsFile::at(SecretsFile::default_path()?);
    match &cli.command {
        Command::Auth {
            command: auth::Command::Login,
        } => auth::login::execute(&secrets, auth::login::prompt_credentials, out),
        Command::Auth {
            command: auth::Command::Logout,
        } => auth::logout::execute(&secrets, &database::default_path()?, &secrets, out),
        Command::Document {
            command: document::Command::Sync(args),
        } => commands::document::sync::execute(
            &secrets,
            &database::default_path()?,
            &secrets,
            |credentials| HttpClient::new(Endpoints::from_env(), credentials),
            &SystemClock,
            args,
            out,
        ),
        Command::Document {
            command: document::Command::Search(args),
        } => commands::document::search::execute(&database::default_path()?, &secrets, args, out),
        Command::Document {
            command: document::Command::Stat(args),
        } => commands::document::stat::execute(&database::default_path()?, &secrets, args, out),
        Command::Document {
            command: document::Command::Status(args),
        } => commands::document::status::execute(
            &secrets,
            &database::default_path()?,
            &secrets,
            |credentials| HttpClient::new(Endpoints::from_env(), credentials),
            args,
            out,
        ),
        Command::Entity {
            command: entity::Command::UpdateClaim(args),
        } => commands::entity::update_claim::execute(
            &secrets,
            |credentials| HttpClient::new(Endpoints::from_env(), credentials),
            args,
            out,
        ),
        Command::Entity {
            command: entity::Command::Search(args),
        } => commands::entity::search::execute(
            &secrets,
            |credentials| HttpClient::new(Endpoints::from_env(), credentials),
            args,
            out,
        ),
        Command::Directory {
            command: directory::Command::French(args),
        } => commands::directory::french::execute(
            &secrets,
            |credentials| HttpClient::new(Endpoints::from_env(), credentials),
            args,
            out,
        ),
    }
}
