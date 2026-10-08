use std::io;
use std::path::PathBuf;

use jiff::Timestamp;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O failure: {0}")]
    Io(#[from] io::Error),

    #[error("cannot read user input: {0}")]
    Prompt(#[from] dialoguer::Error),

    #[error(
        "{} is readable by other users; run `chmod 600` on it, or `iopole auth logout` then `iopole auth login`",
        .0.display()
    )]
    ExposedSecrets(PathBuf),

    #[error("not logged in; run `iopole auth login` first")]
    NotLoggedIn,

    #[error("cannot locate home directory")]
    HomeDir(#[from] etcetera::HomeDirError),

    #[error("database failure: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("cannot generate database key: {0}")]
    KeyGeneration(#[from] getrandom::Error),

    #[error(
        "encryption key for {} is missing from the secrets file; run `iopole auth logout` to delete the database, then log in and run `iopole document sync` again",
        .0.display()
    )]
    MissingDatabaseKey(PathBuf),

    #[error("cannot decrypt {} with the key from the secrets file", .0.display())]
    InvalidDatabaseKey(PathBuf),

    #[error("database schema version {0} is newer than this tool supports")]
    UnsupportedSchema(i64),

    #[error("cannot reach Iopole: {0}")]
    Http(Box<ureq::Error>),

    #[error("unexpected Iopole response: {0}")]
    Payload(#[from] serde_json::Error),

    #[error(
        "Iopole authentication failed with HTTP {0}; check the credentials or run `iopole auth login` again"
    )]
    Authentication(u16),

    #[error("Iopole refused the access token (HTTP {0})")]
    Unauthorized(u16),

    #[error("Iopole rejected the request: {0}")]
    InvalidRequest(String),

    #[error("Iopole rate limit still exceeded after several waits")]
    RateLimited,

    #[error("Iopole server error (HTTP {0})")]
    Server(u16),

    #[error("unexpected Iopole response (HTTP {status}{})", .code.as_ref().map(|code| format!(", code {code}")).unwrap_or_default())]
    UnexpectedStatus { status: u16, code: Option<String> },

    #[error("{0} is not registered with Iopole")]
    UnregisteredIdentifier(String),

    #[error(
        "{identifier_value} matches {count} business entities; pass a more specific identifier, such as a SIRET"
    )]
    AmbiguousIdentifier {
        identifier_value: String,
        count: usize,
    },

    #[error("Iopole knows no business entity {0}")]
    UnknownBusinessEntity(String),

    #[error("business entity {0} is not claimed by this operator")]
    UnclaimedBusinessEntity(String),

    #[error(
        "business entity {0} still has active inbound network registrations; it cannot be restricted to outbound"
    )]
    ActiveInboundRegistrations(String),

    #[error("invalid date: {0}")]
    InvalidDate(#[from] jiff::Error),

    #[error("{0} cannot be represented in an Iopole invoice identifier")]
    InstantOutOfRange(Timestamp),

    #[error("the start of the period ({from}) must be before its end ({to})")]
    EmptyPeriod { from: Timestamp, to: Timestamp },
}

impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        Self::Http(Box::new(err))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
