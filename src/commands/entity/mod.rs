pub(crate) mod register;
pub(crate) mod search;
pub(crate) mod update_claim;

use clap::Subcommand;

use crate::iopole::BusinessEntity;
use crate::{Error, Result};

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Show whether a company is registered with Iopole, and how
    Search(search::Args),
    /// Replace how the operator claims a business entity: its company UID and invoice flow
    UpdateClaim(update_claim::Args),
    /// Publish an identifier of a business entity on the e-invoicing networks
    Register(register::Args),
}

/// Looked up as `entity search` does: a change must not land on a guess.
fn single_entity(
    mut entities: Vec<BusinessEntity>,
    identifier_value: &str,
) -> Result<BusinessEntity> {
    match entities.len() {
        0 => Err(Error::UnregisteredIdentifier(identifier_value.to_owned())),
        1 => Ok(entities.remove(0)),
        count => Err(Error::AmbiguousIdentifier {
            identifier_value: identifier_value.to_owned(),
            count,
        }),
    }
}
