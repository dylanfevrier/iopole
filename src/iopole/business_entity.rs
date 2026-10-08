use serde::Deserialize;

use crate::Result;

const COMPANY_UID_HEADER: &str = "X-Company-UID";

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BusinessEntity {
    pub(crate) name: Option<String>,
    #[serde(rename = "type")]
    pub(crate) entity_type: Option<String>,
    pub(crate) scope: Option<String>,
    pub(crate) country: Option<String>,
    pub(crate) operator_relation: Option<OperatorRelation>,
    #[serde(default)]
    pub(crate) identifiers: Vec<EntityIdentifier>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct OperatorRelation {
    pub(crate) data: Option<OperatorRelationData>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct OperatorRelationData {
    #[serde(default)]
    pub(crate) header: Vec<Header>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct Header {
    pub(crate) key: String,
    pub(crate) value: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EntityIdentifier {
    #[serde(rename = "type")]
    pub(crate) identifier_type: Option<String>,
    pub(crate) scheme: Option<String>,
    pub(crate) value: Option<String>,
    #[serde(default)]
    pub(crate) network_registered: Vec<NetworkRegistration>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NetworkRegistration {
    pub(crate) network_identifier: Option<String>,
    pub(crate) directory_address: Option<String>,
    pub(crate) valid_from: Option<String>,
}

impl BusinessEntity {
    /// Identifier of the company in the operator's own system.
    pub(crate) fn company_uid(&self) -> Option<&str> {
        self.operator_relation
            .as_ref()?
            .data
            .as_ref()?
            .header
            .iter()
            .find(|header| header.key.eq_ignore_ascii_case(COMPANY_UID_HEADER))
            .map(|header| header.value.as_str())
    }
}

#[derive(Deserialize)]
struct EntityPage<T> {
    data: Vec<T>,
}

pub(crate) fn parse_entities(body: &str) -> Result<Vec<BusinessEntity>> {
    Ok(serde_json::from_str::<EntityPage<BusinessEntity>>(body)?.data)
}

/// The entities as Iopole returns them, out of their page.
pub(crate) fn parse_raw_entities(body: &str) -> Result<Vec<serde_json::Value>> {
    Ok(serde_json::from_str::<EntityPage<serde_json::Value>>(body)?.data)
}

pub(crate) fn identifier_query(identifier_value: &str) -> String {
    let escaped = identifier_value.replace('\\', "\\\\").replace('"', "\\\"");
    format!(r#"identifierValue:"{escaped}""#)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iopole::testing::bvs_renovation;

    const OBSERVED_PAGE: &str = r#"{
  "data": [
    {
      "businessEntityId": "0199b69f-fe32-701a-9604-617d13785726",
      "country": "FR",
      "countryIdentifier": { "siren": "980217632" },
      "disclosureStatus": "Public",
      "identifierScheme": "0002",
      "identifierValue": "980217632",
      "identifiers": [
        {
          "businessEntityIdentifierId": "0199b69f-fe32-701a-9604-6669329b91fe",
          "networkRegistered": [
            {
              "directoryAddress": "0225:980217632",
              "directoryId": "019fa93e-2169-75be-bbba-e5ad570ec5dc",
              "isSelfBilling": false,
              "networkId": "c11d52d5-40b8-4d71-83f1-1709cf47e834",
              "networkIdentifier": "PEPPOL_INTERNATIONAL",
              "validFrom": "2026-07-28"
            },
            {
              "directoryAddress": "0225:980217632",
              "directoryId": "019fa93e-1b8b-77a9-8e2b-c29a88f44712",
              "isSelfBilling": false,
              "networkId": "b00d52d5-40b8-4d71-83f1-1709cf47e812",
              "networkIdentifier": "DOMESTIC_FR",
              "validFrom": "2026-07-29"
            }
          ],
          "scheme": "0002",
          "type": "LEGAL_IDENTIFIER",
          "value": "980217632"
        }
      ],
      "name": "BVS RENOVATION",
      "operatorRelation": {
        "data": { "header": [ { "key": "X-Company-UID", "value": "6071797" } ] }
      },
      "scope": "PRIVATE_TAX_PAYER",
      "type": "LEGAL_UNIT",
      "vatRegime": "REAL_MONTHLY_TAX_REGIME"
    }
  ],
  "meta": { "count": 1, "limit": 200, "offset": 0 }
}"#;

    #[test]
    fn observed_payload_reads_as_the_fixture() -> Result<()> {
        assert_eq!(parse_entities(OBSERVED_PAGE)?, [bvs_renovation()]);
        Ok(())
    }

    #[test]
    fn everything_is_optional() -> Result<()> {
        let entities = parse_entities(r#"{"data":[{}]}"#)?;

        assert_eq!(entities, [BusinessEntity::default()]);
        assert_eq!(entities[0].company_uid(), None);
        Ok(())
    }

    #[test]
    fn query_matches_the_exact_identifier_value() {
        assert_eq!(
            identifier_query("980217632"),
            r#"identifierValue:"980217632""#
        );
    }

    #[test]
    fn query_escapes_quotes() {
        assert_eq!(identifier_query(r#"a"b\c"#), r#"identifierValue:"a\"b\\c""#);
    }
}
