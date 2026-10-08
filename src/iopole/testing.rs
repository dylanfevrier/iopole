use jiff::Timestamp;

use super::business_entity::{
    BusinessEntity, EntityIdentifier, Header, NetworkRegistration, OperatorRelation,
    OperatorRelationData,
};
use super::status_history::{
    DocumentReference, RoleCode, StatusCode, StatusEvent, StatusMessage, StatusParty,
    StatusResponse,
};
use super::{BusinessEntityDirectory, InvoiceSearch, InvoiceStatuses, SearchQuery};
use crate::Result;
use crate::invoice::{Invoice, Party, Status};

#[derive(Default)]
pub(crate) struct FakeSearch {
    pub(crate) invoices: Vec<Invoice>,
    pub(crate) queries: Vec<(SearchQuery, u32)>,
}

impl FakeSearch {
    pub(crate) fn with(invoices: Vec<Invoice>) -> Self {
        Self {
            invoices,
            queries: Vec::new(),
        }
    }
}

impl InvoiceSearch for FakeSearch {
    fn search(&mut self, query: &SearchQuery, offset: u32, limit: u32) -> Result<Vec<Invoice>> {
        self.queries.push((*query, offset));
        let (from, to) = (query.from.to_string(), query.to.to_string());
        let mut matching: Vec<_> = self
            .invoices
            .iter()
            .filter(|invoice| invoice.direction == query.direction.as_str())
            .filter(|invoice| from.as_str() <= invoice.id.as_str() && invoice.id < to)
            .cloned()
            .collect();
        matching.sort_by(|a, b| b.id.cmp(&a.id));
        Ok(matching
            .into_iter()
            .skip(usize::try_from(offset).unwrap_or(usize::MAX))
            .take(usize::try_from(limit).unwrap_or(usize::MAX))
            .collect())
    }
}

pub(crate) fn id_at(instant: &str, sequence: u16) -> String {
    let millis = instant
        .parse::<Timestamp>()
        .unwrap_or_else(|err| panic!("{instant}: {err}"))
        .as_millisecond();
    let hex = format!("{millis:012x}");
    format!(
        "{}-{}-7{sequence:03x}-8000-{sequence:012x}",
        &hex[..8],
        &hex[8..]
    )
}

pub(crate) fn invoice(id: String, state: &str, last_date: Option<&str>) -> Invoice {
    Invoice {
        id,
        direction: "INBOUND".to_owned(),
        created_at: String::new(),
        status: Status {
            state: state.to_owned(),
            last_date: last_date.map(str::to_owned),
            ..Status::default()
        },
        number: None,
        amount: None,
        seller: Party::default(),
        buyer: Party::default(),
        raw: "{}".to_owned(),
    }
}

pub(crate) fn inbound(instant: &str, sequence: u16) -> Invoice {
    invoice(id_at(instant, sequence), "VALID", None)
}

pub(crate) fn outbound(instant: &str, sequence: u16) -> Invoice {
    Invoice {
        direction: "OUTBOUND".to_owned(),
        ..inbound(instant, sequence)
    }
}

#[derive(Default)]
pub(crate) struct FakeDirectory {
    pub(crate) entities: Vec<BusinessEntity>,
    pub(crate) searched: Vec<String>,
}

impl BusinessEntityDirectory for FakeDirectory {
    fn find(&mut self, identifier_value: &str) -> Result<Vec<BusinessEntity>> {
        self.searched.push(identifier_value.to_owned());
        Ok(self
            .entities
            .iter()
            .filter(|entity| {
                entity
                    .identifiers
                    .iter()
                    .any(|identifier| identifier.value.as_deref() == Some(identifier_value))
            })
            .cloned()
            .collect())
    }
}

pub(crate) fn bvs_renovation() -> BusinessEntity {
    let some = |value: &str| Some(value.to_owned());
    let network = |network: &str, valid_from: &str| NetworkRegistration {
        network_identifier: some(network),
        directory_address: some("0225:980217632"),
        valid_from: some(valid_from),
    };
    BusinessEntity {
        name: some("BVS RENOVATION"),
        entity_type: some("LEGAL_UNIT"),
        scope: some("PRIVATE_TAX_PAYER"),
        country: some("FR"),
        operator_relation: Some(OperatorRelation {
            data: Some(OperatorRelationData {
                header: vec![Header {
                    key: "X-Company-UID".to_owned(),
                    value: "6071797".to_owned(),
                }],
            }),
        }),
        identifiers: vec![EntityIdentifier {
            identifier_type: some("LEGAL_IDENTIFIER"),
            scheme: some("0002"),
            value: some("980217632"),
            network_registered: vec![
                network("PEPPOL_INTERNATIONAL", "2026-07-28"),
                network("DOMESTIC_FR", "2026-07-29"),
            ],
        }],
    }
}

pub(crate) use super::french_directory::testing::{
    FakeFrenchDirectory, legal_unit_with_suffix, office_with_routing_code,
};

pub(crate) struct FakeStatuses {
    pub(crate) history: Option<Vec<StatusEvent>>,
}

impl InvoiceStatuses for FakeStatuses {
    fn status_history(&mut self, _: &str) -> Result<Option<Vec<StatusEvent>>> {
        Ok(self.history.clone())
    }
}

fn party(name: Option<&str>, siren: Option<&str>, role: &str) -> StatusParty {
    StatusParty {
        name: name.map(str::to_owned),
        siren: siren.map(str::to_owned),
        role_code: Some(RoleCode {
            code: Some(role.to_owned()),
        }),
    }
}

fn status(date: &str, code: &str, network_code: &str, message: StatusMessage) -> StatusEvent {
    StatusEvent {
        date: Some(date.to_owned()),
        status: Some(StatusCode {
            code: Some(code.to_owned()),
            network_code: Some(network_code.to_owned()),
        }),
        json: Some(message),
    }
}

/// As Iopole sent it for an invoice of Leroy Merlin France to BVS RENOVATION.
pub(crate) fn payment_received() -> StatusEvent {
    status(
        "2026-10-05T09:45:07.000Z",
        "PAYMENT_RECEIVED",
        "212",
        StatusMessage {
            issuer: Some(party(Some("VENDEUR"), Some("384560942"), "SELLER")),
            sender: Some(party(None, None, "DEMATERIALIZATION_OPERATOR")),
            responses: vec![StatusResponse {
                document_reference: Some(DocumentReference {
                    issuer: Some(party(None, Some("384560942"), "SELLER")),
                }),
            }],
            recipients: vec![
                party(Some("ACHETEUR"), Some("980217632"), "BUYER"),
                party(Some("PPF"), None, "PPF"),
            ],
        },
    )
}

pub(crate) fn approved() -> StatusEvent {
    status(
        "2026-10-01T14:02:11.000Z",
        "APPROVED",
        "205",
        StatusMessage {
            sender: Some(party(Some("ACHETEUR"), Some("980217632"), "BUYER")),
            ..StatusMessage::default()
        },
    )
}
