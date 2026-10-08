pub(crate) mod login;
pub(crate) mod logout;

use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Store Iopole credentials in a file readable only by you
    Login,
    /// Log out and remove credentials, local database and its encryption key
    Logout,
}
