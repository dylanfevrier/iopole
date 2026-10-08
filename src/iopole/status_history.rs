use serde::Deserialize;

use crate::Result;

const SELLER: &str = "SELLER";
const BUYER: &str = "BUYER";

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusEvent {
    pub(crate) date: Option<String>,
    pub(crate) status: Option<StatusCode>,
    pub(crate) json: Option<StatusMessage>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusCode {
    pub(crate) code: Option<String>,
    pub(crate) network_code: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct StatusMessage {
    pub(crate) issuer: Option<StatusParty>,
    pub(crate) sender: Option<StatusParty>,
    #[serde(default)]
    pub(crate) responses: Vec<StatusResponse>,
    #[serde(default)]
    pub(crate) recipients: Vec<StatusParty>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusResponse {
    pub(crate) document_reference: Option<DocumentReference>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentReference {
    pub(crate) issuer: Option<StatusParty>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusParty {
    pub(crate) name: Option<String>,
    pub(crate) siren: Option<String>,
    pub(crate) role_code: Option<RoleCode>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub(crate) struct RoleCode {
    pub(crate) code: Option<String>,
}

impl StatusParty {
    fn has_role(&self, role: &str) -> bool {
        self.role_code
            .as_ref()
            .and_then(|role_code| role_code.code.as_deref())
            == Some(role)
    }
}

impl StatusMessage {
    fn parties(&self) -> impl Iterator<Item = &StatusParty> {
        let issuers = self
            .responses
            .iter()
            .filter_map(|response| response.document_reference.as_ref()?.issuer.as_ref());
        self.issuer
            .iter()
            .chain(&self.sender)
            .chain(&self.recipients)
            .chain(issuers)
    }
}

/// Statuses carry the parties exchanging them, often under generic names
/// (`VENDEUR`, `ACHETEUR`) but with their SIREN.
fn party<'a>(history: &'a [StatusEvent], role: &str) -> Option<&'a StatusParty> {
    history
        .iter()
        .filter_map(|event| event.json.as_ref())
        .flat_map(StatusMessage::parties)
        .find(|party| party.has_role(role))
}

pub(crate) fn seller(history: &[StatusEvent]) -> Option<&StatusParty> {
    party(history, SELLER)
}

pub(crate) fn buyer(history: &[StatusEvent]) -> Option<&StatusParty> {
    party(history, BUYER)
}

pub(crate) fn parse_status_history(body: &str) -> Result<Vec<StatusEvent>> {
    Ok(serde_json::from_str(body)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iopole::testing::payment_received;

    const OBSERVED: &str = r#"[{
      "invoiceId": "01a10b05-e3fd-74c8-b9cd-ce8e4c012777",
      "statusId": "01a10b06-164d-77d8-8b9f-0fd6abbb07fe",
      "date": "2026-10-05T09:45:07.000Z",
      "destType": "OPERATOR",
      "status": { "code": "PAYMENT_RECEIVED", "networkCode": "212" },
      "xml": "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>",
      "json": {
        "issuer": {
          "name": "VENDEUR", "siren": "384560942",
          "roleCode": { "code": "SELLER", "desc": "Vendeur (B2C)", "value": "SE" },
          "identifiers": [{ "type": "PARTY_IDENTIFIER", "value": "384560942", "scheme": "0002" }]
        },
        "sender": {
          "roleCode": { "code": "DEMATERIALIZATION_OPERATOR", "value": "WK" }
        },
        "profileId": "urn.cpro.gouv.fr:1p0:CDV:invoice",
        "responses": [{
          "additionalData": [{ "type": "MEN", "amount": { "value": 0.04, "currency": "EUR" }, "vatRate": 20 }],
          "documentStatus": { "code": "PAYMENT_RECEIVED", "networkCode": "212" },
          "documentReference": {
            "issuer": {
              "siren": "384560942",
              "roleCode": { "code": "SELLER", "desc": "Vendeur (B2C)", "value": "SE" }
            },
            "typeCode": 380,
            "issueDate": "2026-10-05T00:00:00Z",
            "receiptDate": "2026-10-05T07:44:52Z",
            "issuerAssignedId": "CFR00101826002485"
          }
        }],
        "recipients": [
          {
            "name": "ACHETEUR", "siren": "980217632",
            "roleCode": { "code": "BUYER", "desc": "Acheteur", "value": "BY" },
            "electronicAddress": "0225:980217632"
          },
          { "name": "PPF", "roleCode": { "code": "PPF", "value": "DFH" } }
        ],
        "testIndicator": false,
        "identification": { "id": "CFR00101826002485_380_212_20261005094507" },
        "businessProcessId": "REGULATED"
      }
    }]"#;

    #[test]
    fn observed_payload_reads_as_the_fixture() -> Result<()> {
        assert_eq!(parse_status_history(OBSERVED)?, [payment_received()]);
        Ok(())
    }

    #[test]
    fn finds_the_seller_and_the_buyer_by_their_role() -> Result<()> {
        let documented = parse_status_history(
            r#"[{"json": {
                "sender": {"name": "Sender Company", "siren": "123456789", "roleCode": {"code": "BUYER"}},
                "responses": [{"documentReference": {"issuer":
                    {"name": "Issuer Company", "siren": "987654321", "roleCode": {"code": "SELLER"}}}}],
                "recipients": [{"name": "Recipient Company", "roleCode": {"code": "INTERMEDIARY"}}]
            }}]"#,
        )?;

        assert_eq!(
            seller(&documented).and_then(|party| party.name.as_deref()),
            Some("Issuer Company")
        );
        assert_eq!(
            buyer(&documented).and_then(|party| party.name.as_deref()),
            Some("Sender Company")
        );
        Ok(())
    }

    #[test]
    fn an_invoice_may_have_no_status_yet() -> Result<()> {
        let history = parse_status_history("[]")?;

        assert_eq!(history, []);
        assert_eq!(seller(&history), None);
        Ok(())
    }
}
