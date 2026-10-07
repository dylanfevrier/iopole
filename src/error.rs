use std::io;
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O failure: {0}")]
    Io(#[from] io::Error),

    #[error("cannot read user input: {0}")]
    Prompt(#[from] dialoguer::Error),

    #[error("cannot access system keyring: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("not logged in; run `iopole login` first")]
    NotLoggedIn,

    #[error("cannot locate home directory")]
    HomeDir(#[from] etcetera::HomeDirError),

    #[error("database failure: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("cannot generate database key: {0}")]
    KeyGeneration(#[from] getrandom::Error),

    #[error(
        "encryption key for {} is missing from the system keyring; delete the database and run `iopole sync` again",
        .0.display()
    )]
    MissingDatabaseKey(PathBuf),

    #[error("cannot decrypt {} with the key from the system keyring", .0.display())]
    InvalidDatabaseKey(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;
