use std::fmt;
use std::io::Write;

use crate::Result;
use crate::commands::business_entity;
use crate::iopole::{DirectoryEntry, PostalAddress};
use crate::render::{self, Fields};

const UNNAMED: &str = "(unnamed)";

/// `next_offset` is set when more entries may follow, the directory not
/// telling how many match beyond the page.
pub(super) fn entries(
    out: &mut impl Write,
    entries: &[DirectoryEntry],
    next_offset: Option<u64>,
) -> Result<()> {
    if entries.is_empty() {
        writeln!(out, "No entry of the French directory matches.")?;
        return Ok(());
    }
    render::records(out, entries.iter().map(Entry))?;
    if let Some(next) = next_offset {
        writeln!(out, "\nMore may match: run again with --offset {next}.")?;
    }
    Ok(())
}

pub(super) fn json(out: &mut impl Write, body: &str) -> Result<()> {
    let body: serde_json::Value = serde_json::from_str(body)?;
    writeln!(out, "{}", serde_json::to_string_pretty(&body)?)?;
    Ok(())
}

struct Entry<'a>(&'a DirectoryEntry);

impl fmt::Display for Entry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entry = self.0;
        let mut fields = Fields::default();
        fields.push("Name", entry.name.as_deref().unwrap_or(UNNAMED));
        fields.push(
            "Type",
            business_entity::kind(entry.entry_type.as_deref(), entry.scope.as_deref()),
        );
        fields.push_some("Legal unit", legal_unit(entry));
        fields.push_some("Address", entry.postal_address.as_ref().and_then(one_line));
        writeln!(f, "{fields}")?;
        if entry.identifiers.is_empty() {
            writeln!(f, "No identifier.")
        } else {
            write!(f, "{}", business_entity::identifiers(&entry.identifiers))
        }
    }
}

fn legal_unit(entry: &DirectoryEntry) -> Option<String> {
    let name = entry
        .legal_unit
        .as_ref()?
        .name
        .as_deref()
        .unwrap_or(UNNAMED);
    Some(match &entry.country_identifier.siren {
        Some(siren) => format!("{name} (SIREN {siren})"),
        None => name.to_owned(),
    })
}

fn one_line(address: &PostalAddress) -> Option<String> {
    let locality = [&address.postal_code, &address.city]
        .into_iter()
        .filter_map(|part| non_blank(part.as_deref()))
        .collect::<Vec<_>>()
        .join(" ");
    let parts: Vec<&str> = [
        &address.address_line1,
        &address.address_line2,
        &address.address_line3,
    ]
    .into_iter()
    .filter_map(|line| non_blank(line.as_deref()))
    .chain(non_blank(Some(&locality)))
    .collect();
    (!parts.is_empty()).then(|| parts.join(", "))
}

fn non_blank(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iopole::testing::{legal_unit_with_suffix, office_with_routing_code};

    fn rendered(list: &[DirectoryEntry], next_offset: Option<u64>) -> String {
        let mut out = Vec::new();
        entries(&mut out, list, next_offset).unwrap_or_else(|err| panic!("cannot render: {err}"));
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn shows_legal_unit_and_its_identifiers() {
        assert_eq!(
            rendered(&[legal_unit_with_suffix()], None),
            "\
Name:         Legal unit Example 2
Type:         Legal unit (PRIVATE_TAX_PAYER)

TYPE              IDENTIFIER      NETWORK      DIRECTORY ADDRESS        AVAILABLE SINCE
LEGAL_IDENTIFIER  0002:009460134  -
SUFFIX            0000:SUFFIX_A   DOMESTIC_FR  0225:009460134_SUFFIX_A  2025-08-10
"
        );
    }

    #[test]
    fn shows_office_legal_unit_and_address() {
        assert_eq!(
            rendered(&[office_with_routing_code()], None),
            "\
Name:         Office 2
Type:         Office (SECONDARY)
Legal unit:   Legal unit Example 1 (SIREN 007350101)
Address:      150 RUE DU TEST, 66000 CITY

TYPE               IDENTIFIER           NETWORK      DIRECTORY ADDRESS                       AVAILABLE SINCE
OFFICE_IDENTIFIER  0009:00735010100083  DOMESTIC_FR  0225:007350101_00735010100083           2025-08-10
ROUTING_CODE       0224:ServiceA        DOMESTIC_FR  0225:007350101_00735010100083_ServiceA  2025-08-10
"
        );
    }

    #[test]
    fn lists_every_network_of_an_identifier() {
        let mut entry = legal_unit_with_suffix();
        let mut peppol = entry.identifiers[1].network_registered[0].clone();
        peppol.network_identifier = Some("PEPPOL_INTERNATIONAL".to_owned());
        entry.identifiers[1].network_registered.push(peppol);

        assert!(rendered(&[entry], None).ends_with(
            "\
SUFFIX            0000:SUFFIX_A   DOMESTIC_FR           0225:009460134_SUFFIX_A  2025-08-10
                                  PEPPOL_INTERNATIONAL  0225:009460134_SUFFIX_A  2025-08-10
"
        ));
    }

    #[test]
    fn leaves_out_what_the_directory_omits() {
        let mut entry = office_with_routing_code();
        entry.scope = None;
        entry.legal_unit = None;
        entry.postal_address = Some(PostalAddress {
            city: Some(" ".to_owned()),
            ..PostalAddress::default()
        });
        entry.identifiers.truncate(1);
        entry.identifiers[0].network_registered.clear();

        assert_eq!(
            rendered(&[entry], None),
            "\
Name:         Office 2
Type:         Office

TYPE               IDENTIFIER           NETWORK
OFFICE_IDENTIFIER  0009:00735010100083  -
"
        );
    }

    #[test]
    fn keeps_unknown_entry_types_as_received() {
        let mut entry = legal_unit_with_suffix();
        entry.entry_type = Some("PUBLIC_BODY".to_owned());
        entry.identifiers.clear();

        assert!(
            rendered(&[entry], None)
                .contains("Type:         PUBLIC_BODY (PRIVATE_TAX_PAYER)\n\nNo identifier.\n")
        );
    }

    #[test]
    fn invites_to_the_next_page() {
        assert!(
            rendered(&[legal_unit_with_suffix()], Some(6))
                .ends_with("2025-08-10\n\nMore may match: run again with --offset 6.\n")
        );
    }

    #[test]
    fn reports_no_match() {
        assert_eq!(
            rendered(&[], None),
            "No entry of the French directory matches.\n"
        );
    }

    #[test]
    fn indents_the_raw_response() -> Result<()> {
        let mut out = Vec::new();

        json(&mut out, r#"{"data":[],"meta":{"count":0}}"#)?;

        assert_eq!(
            String::from_utf8_lossy(&out),
            "{\n  \"data\": [],\n  \"meta\": {\n    \"count\": 0\n  }\n}\n"
        );
        Ok(())
    }
}
