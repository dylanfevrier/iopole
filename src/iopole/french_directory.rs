use std::fmt;

use serde::Deserialize;

use super::business_entity::EntityIdentifier;
use crate::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DirectoryEntryType {
    LegalUnit,
    Office,
}

impl DirectoryEntryType {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LegalUnit => "LEGAL_UNIT",
            Self::Office => "OFFICE",
        }
    }

    pub(crate) fn from_code(code: &str) -> Option<Self> {
        [Self::LegalUnit, Self::Office]
            .into_iter()
            .find(|entry_type| entry_type.as_str() == code)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DirectoryQuery {
    pub(crate) siren: Option<String>,
    pub(crate) siret: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) entry_type: Option<DirectoryEntryType>,
}

impl fmt::Display for DirectoryQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let criteria = [
            self.siren.as_deref().map(|siren| ("siren", siren)),
            self.siret.as_deref().map(|siret| ("siret", siret)),
            self.name.as_deref().map(|name| ("name", name)),
            self.entry_type
                .map(|entry_type| ("type", entry_type.as_str())),
        ];
        for (index, (field, value)) in criteria.into_iter().flatten().enumerate() {
            if index > 0 {
                f.write_str(" AND ")?;
            }
            let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
            write!(f, r#"{field}:"{escaped}""#)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectoryEntry {
    pub(crate) name: Option<String>,
    #[serde(rename = "type")]
    pub(crate) entry_type: Option<String>,
    pub(crate) scope: Option<String>,
    pub(crate) country: Option<String>,
    #[serde(default)]
    pub(crate) country_identifier: CountryIdentifier,
    pub(crate) postal_address: Option<PostalAddress>,
    pub(crate) legal_unit: Option<LegalUnit>,
    #[serde(default)]
    pub(crate) identifiers: Vec<EntityIdentifier>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct CountryIdentifier {
    pub(crate) siren: Option<String>,
    pub(crate) siret: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PostalAddress {
    pub(crate) address_line1: Option<String>,
    pub(crate) address_line2: Option<String>,
    pub(crate) address_line3: Option<String>,
    pub(crate) postal_code: Option<String>,
    pub(crate) city: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct LegalUnit {
    pub(crate) name: Option<String>,
}

/// The directory replies 404 when nothing matches.
pub(crate) const EMPTY_PAGE: &str = r#"{"data":[]}"#;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectoryPage {
    pub(crate) entries: Vec<DirectoryEntry>,
    pub(crate) body: String,
}

#[derive(Deserialize)]
struct RawPage {
    #[serde(default)]
    data: Vec<DirectoryEntry>,
}

/// `meta.count` is the size of the page, not the number of matches.
pub(crate) fn parse_directory_page(body: String) -> Result<DirectoryPage> {
    let entries = serde_json::from_str::<RawPage>(&body)?.data;
    Ok(DirectoryPage { entries, body })
}

#[cfg(test)]
pub(crate) mod testing {
    use super::super::FrenchDirectory;
    use super::super::business_entity::{EntityIdentifier, NetworkRegistration};
    use super::{
        CountryIdentifier, DirectoryEntry, DirectoryPage, DirectoryQuery, EMPTY_PAGE, LegalUnit,
        PostalAddress,
    };
    use crate::Result;

    #[derive(Default)]
    pub(crate) struct FakeFrenchDirectory {
        pub(crate) entries: Vec<DirectoryEntry>,
        pub(crate) body: Option<String>,
        pub(crate) searched: Vec<(DirectoryQuery, u32, u32)>,
    }

    impl FrenchDirectory for FakeFrenchDirectory {
        fn search_french(
            &mut self,
            query: &DirectoryQuery,
            offset: u32,
            limit: u32,
        ) -> Result<DirectoryPage> {
            self.searched.push((query.clone(), offset, limit));
            Ok(DirectoryPage {
                entries: self
                    .entries
                    .iter()
                    .skip(usize::try_from(offset).unwrap_or(usize::MAX))
                    .take(usize::try_from(limit).unwrap_or(usize::MAX))
                    .cloned()
                    .collect(),
                body: self.body.clone().unwrap_or_else(|| EMPTY_PAGE.to_owned()),
            })
        }
    }

    fn identifier(
        identifier_type: &str,
        scheme: &str,
        value: &str,
        directory_addresses: &[&str],
    ) -> EntityIdentifier {
        EntityIdentifier {
            identifier_type: Some(identifier_type.to_owned()),
            scheme: Some(scheme.to_owned()),
            value: Some(value.to_owned()),
            network_registered: directory_addresses
                .iter()
                .map(|address| NetworkRegistration {
                    network_identifier: Some("DOMESTIC_FR".to_owned()),
                    directory_address: Some((*address).to_owned()),
                    valid_from: Some("2025-08-10".to_owned()),
                })
                .collect(),
        }
    }

    pub(crate) fn legal_unit_with_suffix() -> DirectoryEntry {
        DirectoryEntry {
            name: Some("Legal unit Example 2".to_owned()),
            entry_type: Some("LEGAL_UNIT".to_owned()),
            scope: Some("PRIVATE_TAX_PAYER".to_owned()),
            country_identifier: CountryIdentifier {
                siren: Some("009460134".to_owned()),
                siret: None,
            },
            identifiers: vec![
                identifier("LEGAL_IDENTIFIER", "0002", "009460134", &[]),
                identifier("SUFFIX", "0000", "SUFFIX_A", &["0225:009460134_SUFFIX_A"]),
            ],
            ..DirectoryEntry::default()
        }
    }

    pub(crate) fn office_with_routing_code() -> DirectoryEntry {
        DirectoryEntry {
            name: Some("Office 2".to_owned()),
            entry_type: Some("OFFICE".to_owned()),
            scope: Some("SECONDARY".to_owned()),
            country: Some("FR".to_owned()),
            country_identifier: CountryIdentifier {
                siren: Some("007350101".to_owned()),
                siret: Some("00735010100083".to_owned()),
            },
            postal_address: Some(PostalAddress {
                address_line1: Some("150 RUE DU TEST".to_owned()),
                postal_code: Some("66000".to_owned()),
                city: Some("CITY".to_owned()),
                ..PostalAddress::default()
            }),
            legal_unit: Some(LegalUnit {
                name: Some("Legal unit Example 1".to_owned()),
            }),
            identifiers: vec![
                identifier(
                    "OFFICE_IDENTIFIER",
                    "0009",
                    "00735010100083",
                    &["0225:007350101_00735010100083"],
                ),
                identifier(
                    "ROUTING_CODE",
                    "0224",
                    "ServiceA",
                    &["0225:007350101_00735010100083_ServiceA"],
                ),
            ],
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const LEGAL_UNIT_PAGE: &str = r#"{
  "data": [
    {
      "businessEntityId": "0193acea-5c97-7665-ada2-c57495af678c",
      "name": "Legal unit Example 2",
      "type": "LEGAL_UNIT",
      "scope": "PRIVATE_TAX_PAYER",
      "identifierScheme": "0002",
      "identifierValue": "009460134",
      "countryIdentifier": { "siren": "009460134" },
      "identifiers": [
        {
          "businessEntityIdentifierId": "0193acea-5c97-7665-ada2-cbe599f72cba",
          "type": "LEGAL_IDENTIFIER",
          "scheme": "0002",
          "value": "009460134"
        },
        {
          "businessEntityIdentifierId": "0193acea-5c97-7665-ada2-cbe599f65ebb",
          "type": "SUFFIX",
          "scheme": "0000",
          "value": "SUFFIX_A",
          "networkRegistered": [
            {
              "directoryId": "0123c00e-1d01-7136-a979-7c3910409eb2",
              "networkId": "b00d52d5-40b8-4d71-83f1-1709cf47e812",
              "networkIdentifier": "DOMESTIC_FR",
              "directoryAddress": "0225:009460134_SUFFIX_A",
              "validFrom": "2025-08-10"
            }
          ]
        }
      ]
    }
  ],
  "meta": { "offset": 0, "limit": 1, "count": 1 }
}"#;

    const OFFICE_PAGE: &str = r#"{
  "data": [
    {
      "businessEntityId": "0193acea-5d3b-7665-ada5-8441740e0b70",
      "name": "Head Office 1",
      "type": "OFFICE",
      "scope": "PRIMARY",
      "country": "FR",
      "identifierScheme": "0009",
      "identifierValue": "00735010100015",
      "countryIdentifier": { "siren": "007350101", "siret": "00735010100015" },
      "postalAddress": {
        "city": "CITY",
        "postalCode": "34470",
        "addressLine1": "1 RUE DU TEST",
        "addressLine2": "BAT B"
      },
      "legalUnit": {
        "businessEntityId": "0193acea-5d3a-7665-ada5-64ffee5ab779",
        "name": "Legal unit Example 1"
      },
      "identifiers": [],
      "meta": { "offset": 0, "limit": 1, "count": 1 }
    }
  ]
}"#;

    #[test]
    fn documented_legal_unit_reads_as_the_fixture() -> Result<()> {
        let entries = parse_directory_page(LEGAL_UNIT_PAGE.to_owned())?.entries;

        assert_eq!(entries, [testing::legal_unit_with_suffix()]);
        Ok(())
    }

    #[test]
    fn reads_office_address_and_legal_unit() -> Result<()> {
        let entries = parse_directory_page(OFFICE_PAGE.to_owned())?.entries;

        let entry = &entries[0];
        assert_eq!(
            entry.country_identifier.siret.as_deref(),
            Some("00735010100015")
        );
        assert_eq!(
            entry
                .legal_unit
                .as_ref()
                .and_then(|unit| unit.name.as_deref()),
            Some("Legal unit Example 1")
        );
        let address = entry.postal_address.clone().unwrap_or_default();
        assert_eq!(address.address_line1.as_deref(), Some("1 RUE DU TEST"));
        assert_eq!(address.address_line2.as_deref(), Some("BAT B"));
        assert_eq!(address.postal_code.as_deref(), Some("34470"));
        assert_eq!(address.city.as_deref(), Some("CITY"));
        Ok(())
    }

    #[test]
    fn everything_is_optional() -> Result<()> {
        let entries = parse_directory_page(r#"{"data":[{}]}"#.to_owned())?.entries;

        assert_eq!(entries, [DirectoryEntry::default()]);
        Ok(())
    }

    #[test]
    fn an_empty_page_has_no_entry() -> Result<()> {
        assert_eq!(parse_directory_page(EMPTY_PAGE.to_owned())?.entries, []);
        Ok(())
    }

    #[test]
    fn query_joins_criteria() {
        let query = DirectoryQuery {
            siren: Some("007350101".to_owned()),
            name: Some("OBAT".to_owned()),
            entry_type: Some(DirectoryEntryType::Office),
            ..DirectoryQuery::default()
        };

        assert_eq!(
            query.to_string(),
            r#"siren:"007350101" AND name:"OBAT" AND type:"OFFICE""#
        );
    }

    #[test]
    fn query_escapes_phrases() {
        let query = DirectoryQuery {
            name: Some(r#"A "B" \C"#.to_owned()),
            ..DirectoryQuery::default()
        };

        assert_eq!(query.to_string(), r#"name:"A \"B\" \\C""#);
    }

    #[test]
    fn query_quotes_wildcards_too() {
        let query = DirectoryQuery {
            name: Some("BVS RENO*".to_owned()),
            ..DirectoryQuery::default()
        };

        assert_eq!(query.to_string(), r#"name:"BVS RENO*""#);
    }
}
