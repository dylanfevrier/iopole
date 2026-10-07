use rusqlite::{Connection, params_from_iter};

use crate::Result;
use crate::invoice::Direction;
use crate::iopole::UuidBound;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Seller,
    Buyer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvoiceFilter {
    pub(crate) identifier_value: String,
    pub(crate) role: Option<Role>,
    pub(crate) directions: Vec<Direction>,
    pub(crate) states: Vec<String>,
    pub(crate) statuses: Vec<String>,
    pub(crate) from: Option<UuidBound>,
    pub(crate) to: Option<UuidBound>,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InvoiceRow {
    pub(crate) id: String,
    pub(crate) direction: String,
    pub(crate) created_at: String,
    pub(crate) number: Option<String>,
    pub(crate) amount: Option<String>,
    pub(crate) seller_name: Option<String>,
    pub(crate) buyer_name: Option<String>,
    pub(crate) state: String,
    pub(crate) last_status_code: Option<String>,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Page {
    pub(crate) invoices: Vec<InvoiceRow>,
    pub(crate) total: usize,
}

/// Most recent invoices first.
pub(crate) fn find(connection: &Connection, filter: &InvoiceFilter) -> Result<Page> {
    let (conditions, params) = conditions(filter);
    let sql = format!(
        "SELECT invoice_id, direction, created_at, number, amount, seller_name, buyer_name,
                state, last_status_code, count(*) OVER ()
         FROM invoice WHERE {} ORDER BY invoice_id DESC LIMIT {}",
        conditions.join(" AND "),
        filter.limit,
    );

    let mut page = Page::default();
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query(params_from_iter(params))?;
    while let Some(row) = rows.next()? {
        page.total = usize::try_from(row.get::<_, i64>(9)?).unwrap_or_default();
        page.invoices.push(InvoiceRow {
            id: row.get(0)?,
            direction: row.get(1)?,
            created_at: row.get(2)?,
            number: row.get(3)?,
            amount: row.get(4)?,
            seller_name: row.get(5)?,
            buyer_name: row.get(6)?,
            state: row.get(7)?,
            last_status_code: row.get(8)?,
        });
    }
    Ok(page)
}

fn conditions(filter: &InvoiceFilter) -> (Vec<String>, Vec<String>) {
    let mut conditions = vec![
        match filter.role {
            Some(Role::Seller) => "seller_identifier = ?",
            Some(Role::Buyer) => "buyer_identifier = ?",
            None => "(seller_identifier = ?1 OR buyer_identifier = ?1)",
        }
        .to_owned(),
    ];
    let mut params = vec![filter.identifier_value.clone()];

    let directions: Vec<_> = filter
        .directions
        .iter()
        .map(|direction| direction.as_str().to_owned())
        .collect();
    for (column, values) in [
        ("direction", &directions),
        ("state", &filter.states),
        ("last_status_code", &filter.statuses),
    ] {
        if !values.is_empty() {
            conditions.push(format!(
                "{column} IN ({})",
                vec!["?"; values.len()].join(", ")
            ));
            params.extend(values.iter().cloned());
        }
    }
    if let Some(from) = filter.from {
        conditions.push("invoice_id >= ?".to_owned());
        params.push(from.to_string());
    }
    if let Some(to) = filter.to {
        conditions.push("invoice_id < ?".to_owned());
        params.push(to.to_string());
    }
    (conditions, params)
}

#[cfg(test)]
mod tests;
