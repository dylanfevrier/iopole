use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O failure: {0}")]
    Io(#[from] io::Error),

    #[error("cannot read user input: {0}")]
    Prompt(#[from] dialoguer::Error),

    #[error("cannot access system keyring: {0}")]
    Keyring(#[from] keyring::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
