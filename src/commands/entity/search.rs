use std::io::Write;

use crate::cli::EntitySearchArgs;
use crate::credentials::{CredentialStore, Credentials};
use crate::iopole::{BusinessEntity, BusinessEntityDirectory};
use crate::{Error, Result};

const LABEL_WIDTH: usize = 16;

pub(crate) fn execute<D: BusinessEntityDirectory>(
    credentials: &impl CredentialStore,
    connect: impl FnOnce(Credentials) -> D,
    args: &EntitySearchArgs,
    out: &mut impl Write,
) -> Result<()> {
    let credentials = credentials.load()?.ok_or(Error::NotLoggedIn)?;
    let entities = connect(credentials).find(&args.identifier_value)?;

    let value = &args.identifier_value;
    match entities.len() {
        0 => writeln!(out, "{value} is not registered with Iopole.")?,
        1 => writeln!(out, "{value}: 1 business entity registered with Iopole.")?,
        count => writeln!(
            out,
            "{value}: {count} business entities registered with Iopole."
        )?,
    }
    for entity in &entities {
        writeln!(out)?;
        render(entity, out)?;
    }
    Ok(())
}

fn render(entity: &BusinessEntity, out: &mut impl Write) -> Result<()> {
    writeln!(out, "{}", entity.name.as_deref().unwrap_or("(unnamed)"))?;
    field(out, "Company UID", entity.company_uid().unwrap_or("-"))?;
    let legal_identifier = entity.legal_identifier().and_then(|identifier| {
        match (identifier.scheme.as_deref(), identifier.value.as_deref()?) {
            (Some(scheme), value) => Some(format!("{scheme}:{value}")),
            (None, value) => Some(value.to_owned()),
        }
    });
    field(
        out,
        "Legal identifier",
        legal_identifier.as_deref().unwrap_or("-"),
    )?;
    field(out, "Country", entity.country.as_deref().unwrap_or("-"))?;

    let networks: Vec<[&str; 3]> = entity
        .network_registrations()
        .map(|network| {
            [
                network.network_identifier.as_deref().unwrap_or("-"),
                network.directory_address.as_deref().unwrap_or("-"),
                network.valid_from.as_deref().unwrap_or("-"),
            ]
        })
        .collect();
    if networks.is_empty() {
        return field(out, "Networks", "none");
    }
    let network_width = networks
        .iter()
        .map(|[network, ..]| network.len())
        .max()
        .unwrap_or(0);
    let address_width = networks
        .iter()
        .map(|[_, address, _]| address.len())
        .max()
        .unwrap_or(0);
    writeln!(out, "  Networks")?;
    for [network, address, valid_from] in networks {
        writeln!(
            out,
            "    {network:<network_width$}  {address:<address_width$}  since {valid_from}"
        )?;
    }
    Ok(())
}

fn field(out: &mut impl Write, label: &str, value: &str) -> Result<()> {
    writeln!(out, "  {label:<LABEL_WIDTH$}  {value}")?;
    Ok(())
}

#[cfg(test)]
mod tests;
