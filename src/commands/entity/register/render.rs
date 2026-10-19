use std::io::Write;

use crate::Result;
use crate::iopole::{BusinessEntity, Network, Registration};
use crate::render::{Fields, MISSING};

pub(super) fn entity(
    out: &mut impl Write,
    entity: &BusinessEntity,
    scheme: &str,
    identifier_value: &str,
) -> Result<()> {
    let mut fields = Fields::default();
    fields.push("Name", entity.name.as_deref().unwrap_or(MISSING));
    fields.push("Identifier", format!("{scheme}:{identifier_value}"));
    writeln!(out, "{fields}")?;
    Ok(())
}

pub(super) fn registration(
    out: &mut impl Write,
    network: Network,
    registration: Registration,
) -> Result<()> {
    let outcome = match registration {
        Registration::Registered => "registered",
        Registration::AlreadyUpToDate => "already up to date",
    };
    writeln!(out, "{}: {outcome}", network.as_str())?;
    Ok(())
}
