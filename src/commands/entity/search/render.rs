use std::fmt;
use std::io::Write;

use crate::Result;
use crate::iopole::BusinessEntity;
use crate::render::{self, Fields, MISSING, Table};

const UNNAMED: &str = "(unnamed)";
const LABELS: [&str; 4] = ["Name", "Company UID", "Legal identifier", "Country"];
const NETWORK_COLUMNS: [&str; 3] = ["NETWORK", "DIRECTORY ADDRESS", "VALID FROM"];

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
        let mut fields = Fields::aligned_on(&LABELS);
        fields.push("Name", entity.name.as_deref().unwrap_or(UNNAMED));
        fields.push("Company UID", entity.company_uid().unwrap_or(MISSING));
        fields.push(
            "Legal identifier",
            legal_identifier(entity).unwrap_or_else(|| MISSING.to_owned()),
        );
        fields.push("Country", entity.country.as_deref().unwrap_or(MISSING));
        writeln!(f, "{fields}")?;

        if entity.network_registrations().next().is_none() {
            return writeln!(f, "No network registration.");
        }
        write!(f, "{}", networks(entity))
    }
}

fn legal_identifier(entity: &BusinessEntity) -> Option<String> {
    let identifier = entity.legal_identifier()?;
    let value = identifier.value.as_deref()?;
    Some(match &identifier.scheme {
        Some(scheme) => format!("{scheme}:{value}"),
        None => value.to_owned(),
    })
}

fn networks(entity: &BusinessEntity) -> Table {
    let mut table = Table::new(&NETWORK_COLUMNS);
    for network in entity.network_registrations() {
        table.push([
            cell(network.network_identifier.as_deref()),
            cell(network.directory_address.as_deref()),
            network.valid_from.clone().unwrap_or_default(),
        ]);
    }
    table
}

fn cell(value: Option<&str>) -> String {
    value.unwrap_or(MISSING).to_owned()
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
Name:              BVS RENOVATION
Company UID:       6071797
Legal identifier:  0002:980217632
Country:           FR

NETWORK               DIRECTORY ADDRESS  VALID FROM
PEPPOL_INTERNATIONAL  0225:980217632     2026-07-28
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
Name:              BVS RENOVATION
Company UID:       -
Legal identifier:  0002:980217632
Country:           FR

No network registration.
"
        );
    }

    #[test]
    fn separates_entities_with_a_rule() {
        let mut other = bvs_renovation();
        other.name = Some("BVS RENOVATION BIS".to_owned());

        let output = rendered(&[bvs_renovation(), other]);

        assert!(output.contains(&format!(
            "2026-07-29\n\n{}\n\nName:              BVS RENOVATION BIS\n",
            "=".repeat(64)
        )));
    }

    #[test]
    fn hides_valid_from_when_omitted() {
        let mut entity = bvs_renovation();
        for network in &mut entity.identifiers[0].network_registered {
            network.valid_from = None;
        }

        assert!(rendered(&[entity]).ends_with(
            "\
NETWORK               DIRECTORY ADDRESS
PEPPOL_INTERNATIONAL  0225:980217632
DOMESTIC_FR           0225:980217632
"
        ));
    }

    #[test]
    fn reports_unregistered_company() {
        assert_eq!(rendered(&[]), "980217632 is not registered with Iopole.\n");
    }
}
