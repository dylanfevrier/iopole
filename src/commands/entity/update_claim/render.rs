use std::io::Write;

use super::Args;
use crate::Result;
use crate::iopole::BusinessEntity;
use crate::render::{Fields, MISSING};

pub(super) fn updated(
    out: &mut impl Write,
    entity: &BusinessEntity,
    business_entity_id: &str,
    args: &Args,
) -> Result<()> {
    let mut fields = Fields::default();
    fields.push("Name", entity.name.as_deref().unwrap_or(MISSING));
    fields.push("ID", business_entity_id);
    fields.push("Company UID", args.company_uid.as_str());
    fields.push("Direction", args.direction.label());
    write!(out, "Claim updated.\n\n{fields}")?;
    Ok(())
}
