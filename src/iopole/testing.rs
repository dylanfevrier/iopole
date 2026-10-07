use jiff::Timestamp;

use super::{InvoiceSearch, SearchQuery};
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
