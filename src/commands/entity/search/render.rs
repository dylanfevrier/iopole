use std::fmt;
use std::io::Write;

use crate::Result;
use crate::commands::business_entity;
use crate::iopole::BusinessEntity;
use crate::render::{self, Fields, MISSING};

const UNNAMED: &str = "(unnamed)";

/// The entities as Iopole returns them, pretty-printed.
pub(super) fn json(out: &mut impl Write, entities: &[serde_json::Value]) -> Result<()> {
    serde_json::to_writer_pretty(&mut *out, entities)?;
    writeln!(out)?;
    Ok(())
}

pub(super) fn entities(
    out: &mut impl Write,
    identifier_value: &str,
    entities: &[BusinessEntity],
) -> Result<()> {
    if entities.is_empty() {
        writeln!(out, "{identifier_value} is not registered with Iopole.")?;
        return Ok(());
    }
    render::records(out, entities.iter().map(Entity))?;
    Ok(())
}

struct Entity<'a>(&'a BusinessEntity);

impl fmt::Display for Entity<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entity = self.0;
        let mut fields = Fields::default();
        fields.push("Name", entity.name.as_deref().unwrap_or(UNNAMED));
        fields.push(
            "Type",
            business_entity::kind(entity.entity_type.as_deref(), entity.scope.as_deref()),
        );
        fields.push("Company UID", entity.company_uid().unwrap_or(MISSING));
        fields.push("Country", entity.country.as_deref().unwrap_or(MISSING));
        writeln!(f, "{fields}")?;
        if entity.identifiers.is_empty() {
            writeln!(f, "No identifier.")
        } else {
            write!(f, "{}", business_entity::identifiers(&entity.identifiers))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iopole::testing::bvs_renovation;

    fn rendered(list: &[BusinessEntity]) -> String {
        let mut out = Vec::new();
        entities(&mut out, "980217632", list).unwrap_or_else(|err| panic!("cannot render: {err}"));
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn shows_company_uid_legal_identifier_and_networks() {
        assert_eq!(
            rendered(&[bvs_renovation()]),
            "\
Name:         BVS RENOVATION
Type:         Legal unit (PRIVATE_TAX_PAYER)
Company UID:  6071797
Country:      FR

TYPE              IDENTIFIER      NETWORK               DIRECTORY ADDRESS  AVAILABLE SINCE
LEGAL_IDENTIFIER  0002:980217632  PEPPOL_INTERNATIONAL  0225:980217632     2026-07-28
                                  DOMESTIC_FR           0225:980217632     2026-07-29
"
        );
    }

    #[test]
    fn shows_entities_without_network_registration() {
        let mut entity = bvs_renovation();
        entity.operator_relation = None;
        entity.identifiers[0].network_registered.clear();

        assert_eq!(
            rendered(&[entity]),
            "\
Name:         BVS RENOVATION
Type:         Legal unit (PRIVATE_TAX_PAYER)
Company UID:  -
Country:      FR

TYPE              IDENTIFIER      NETWORK
LEGAL_IDENTIFIER  0002:980217632  -
"
        );
    }

    #[test]
    fn separates_entities_with_a_rule() {
        let mut other = bvs_renovation();
        other.name = Some("BVS RENOVATION BIS".to_owned());

        let output = rendered(&[bvs_renovation(), other]);

        assert!(output.contains(&format!(
            "2026-07-29\n\n{}\n\nName:         BVS RENOVATION BIS\n",
            "=".repeat(64)
        )));
    }

    #[test]
    fn hides_availability_when_omitted() {
        let mut entity = bvs_renovation();
        for network in &mut entity.identifiers[0].network_registered {
            network.valid_from = None;
        }

        assert!(rendered(&[entity]).ends_with(
            "\
TYPE              IDENTIFIER      NETWORK               DIRECTORY ADDRESS
LEGAL_IDENTIFIER  0002:980217632  PEPPOL_INTERNATIONAL  0225:980217632
                                  DOMESTIC_FR           0225:980217632
"
        ));
    }

    #[test]
    fn reports_unregistered_company() {
        assert_eq!(rendered(&[]), "980217632 is not registered with Iopole.\n");
    }
}
