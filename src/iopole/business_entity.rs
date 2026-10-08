use serde::Deserialize;

use crate::Result;

const COMPANY_UID_HEADER: &str = "X-Company-UID";
const LEGAL_IDENTIFIER: &str = "LEGAL_IDENTIFIER";

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BusinessEntity {
    pub(crate) name: Option<String>,
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

    pub(crate) fn legal_identifier(&self) -> Option<&EntityIdentifier> {
        self.identifiers
            .iter()
            .find(|identifier| identifier.identifier_type.as_deref() == Some(LEGAL_IDENTIFIER))
    }

    pub(crate) fn network_registrations(&self) -> impl Iterator<Item = &NetworkRegistration> {
        self.identifiers
            .iter()
            .flat_map(|identifier| &identifier.network_registered)
    }
}

#[derive(Deserialize)]
struct EntityPage {
    data: Vec<BusinessEntity>,
}

pub(crate) fn parse_entities(body: &str) -> Result<Vec<BusinessEntity>> {
    Ok(serde_json::from_str::<EntityPage>(body)?.data)
}

pub(crate) fn identifier_query(identifier_value: &str) -> String {
    let escaped = identifier_value.replace('\\', "\\\\").replace('"', "\\\"");
    format!(r#"identifierValue:"{escaped}""#)
}

#[cfg(test)]
mod tests;
