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
    /// Every company when `None`.
    pub(crate) identifier_value: Option<String>,
    /// Only applies to `identifier_value`.
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
    /// Value of the electronic address, without its scheme (SIREN, BCE...).
    pub(crate) seller_identifier: Option<String>,
    pub(crate) buyer_name: Option<String>,
    pub(crate) buyer_identifier: Option<String>,
    pub(crate) state: String,
    pub(crate) last_status_code: Option<String>,
    /// The invoice as Iopole returned it when synchronized.
    pub(crate) raw: String,
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Page {
    pub(crate) invoices: Vec<InvoiceRow>,
    pub(crate) total: usize,
}

pub(crate) fn find(connection: &Connection, filter: &InvoiceFilter) -> Result<Page> {
    let (conditions, params) = conditions(filter);
    let sql = format!(
        "SELECT invoice_id, direction, created_at, number, amount, seller_name, buyer_name,
                state, last_status_code, raw, seller_identifier, buyer_identifier,
                count(*) OVER ()
         FROM invoice {} ORDER BY invoice_id DESC LIMIT {}",
        where_clause(&conditions),
        filter.limit,
    );

    let mut page = Page::default();
    let mut statement = connection.prepare(&sql)?;
    let mut rows = statement.query(params_from_iter(params))?;
    while let Some(row) = rows.next()? {
        page.total = usize::try_from(row.get::<_, i64>(12)?).unwrap_or_default();
        page.invoices.push(InvoiceRow {
            id: row.get(0)?,
            direction: row.get(1)?,
            created_at: row.get(2)?,
            number: row.get(3)?,
            amount: row.get(4)?,
            seller_name: row.get(5)?,
            seller_identifier: row.get(10)?,
            buyer_name: row.get(6)?,
            buyer_identifier: row.get(11)?,
            state: row.get(7)?,
            last_status_code: row.get(8)?,
            raw: row.get(9)?,
        });
    }
    Ok(page)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StateCount {
    pub(crate) state: String,
    pub(crate) last_status_code: Option<String>,
    pub(crate) count: usize,
}

/// Synchronized invoices of these directions counted by state, then by last
/// status.
pub(crate) fn count_by_state(
    connection: &Connection,
    directions: &[Direction],
) -> Result<Vec<StateCount>> {
    let mut statement = connection.prepare(&format!(
        "SELECT state, last_status_code, count(*) FROM invoice
         WHERE direction IN ({})
         GROUP BY state, last_status_code",
        vec!["?"; directions.len()].join(", ")
    ))?;
    let counts = statement
        .query_map(
            params_from_iter(directions.iter().map(|direction| direction.as_str())),
            |row| {
                Ok(StateCount {
                    state: row.get(0)?,
                    last_status_code: row.get(1)?,
                    count: usize::try_from(row.get::<_, i64>(2)?).unwrap_or_default(),
                })
            },
        )?
        .collect::<rusqlite::Result<_>>()?;
    Ok(counts)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct InvoiceParties {
    pub(crate) seller_name: Option<String>,
    pub(crate) seller_identifier: Option<String>,
    pub(crate) buyer_name: Option<String>,
    pub(crate) buyer_identifier: Option<String>,
}

pub(crate) fn parties(connection: &Connection, invoice_id: &str) -> Result<Option<InvoiceParties>> {
    let mut statement = connection.prepare(
        "SELECT seller_name, seller_identifier, buyer_name, buyer_identifier
         FROM invoice WHERE invoice_id = ?1",
    )?;
    let mut rows = statement.query([invoice_id])?;
    let Some(row) = rows.next()? else {
        return Ok(None);
    };
    Ok(Some(InvoiceParties {
        seller_name: row.get(0)?,
        seller_identifier: row.get(1)?,
        buyer_name: row.get(2)?,
        buyer_identifier: row.get(3)?,
    }))
}

fn where_clause(conditions: &[String]) -> String {
    if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    }
}

fn conditions(filter: &InvoiceFilter) -> (Vec<String>, Vec<String>) {
    let mut conditions = Vec::new();
    let mut params = Vec::new();
    // First, so that `?1` is the identifier value.
    if let Some(identifier_value) = &filter.identifier_value {
        conditions.push(
            match filter.role {
                Some(Role::Seller) => "seller_identifier = ?",
                Some(Role::Buyer) => "buyer_identifier = ?",
                None => "(seller_identifier = ?1 OR buyer_identifier = ?1)",
            }
            .to_owned(),
        );
        params.push(identifier_value.clone());
    }

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
mod tests {
    use super::*;
    use crate::database::{
        self,
        testing::{InMemoryKeyStore, InvoiceRecord},
    };

    struct Fixture {
        connection: Connection,
        _tmp: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Result<Self> {
            let tmp = tempfile::tempdir()?;
            let connection =
                database::open(&tmp.path().join("iopole.db"), &InMemoryKeyStore::default())?;
            Ok(Self {
                connection,
                _tmp: tmp,
            })
        }

        fn insert(
            &self,
            id: &str,
            direction: &str,
            seller: &str,
            buyer: &str,
            state: &str,
            status: Option<&str>,
        ) -> Result<()> {
            InvoiceRecord {
                id,
                direction,
                seller_identifier: Some(seller),
                buyer_identifier: Some(buyer),
                state,
                last_status_code: status,
                ..InvoiceRecord::default()
            }
            .insert(&self.connection)
        }
    }

    const IDENTIFIER: &str = "352862346";
    const OTHER: &str = "517657334";

    fn filter() -> InvoiceFilter {
        InvoiceFilter {
            identifier_value: Some(IDENTIFIER.to_owned()),
            role: None,
            directions: Vec::new(),
            states: Vec::new(),
            statuses: Vec::new(),
            from: None,
            to: None,
            limit: 50,
        }
    }

    fn ids(page: &Page) -> Vec<&str> {
        page.invoices
            .iter()
            .map(|invoice| invoice.id.as_str())
            .collect()
    }

    fn populated() -> Result<Fixture> {
        let fixture = Fixture::new()?;
        fixture.insert(
            "01a1",
            "INBOUND",
            IDENTIFIER,
            OTHER,
            "DELIVERED",
            Some("REJECTED"),
        )?;
        fixture.insert(
            "01a2",
            "OUTBOUND",
            OTHER,
            IDENTIFIER,
            "VALID",
            Some("APPROVED"),
        )?;
        fixture.insert("01a3", "INBOUND", IDENTIFIER, OTHER, "INVALID", None)?;
        fixture.insert("01a4", "INBOUND", OTHER, OTHER, "DELIVERED", None)?;
        Ok(fixture)
    }

    #[test]
    fn finds_identifier_as_seller_or_buyer_most_recent_first() -> Result<()> {
        let fixture = populated()?;

        let page = find(&fixture.connection, &filter())?;

        assert_eq!(ids(&page), ["01a3", "01a2", "01a1"]);
        assert_eq!(page.total, 3);
        Ok(())
    }

    #[test]
    fn finds_every_invoice_without_identifier() -> Result<()> {
        let fixture = populated()?;
        let everyone = InvoiceFilter {
            identifier_value: None,
            ..filter()
        };

        assert_eq!(
            ids(&find(&fixture.connection, &everyone)?),
            ["01a4", "01a3", "01a2", "01a1"]
        );
        Ok(())
    }

    #[test]
    fn filters_without_identifier() -> Result<()> {
        let fixture = populated()?;
        let delivered = InvoiceFilter {
            identifier_value: None,
            states: vec!["DELIVERED".to_owned()],
            ..filter()
        };

        assert_eq!(
            ids(&find(&fixture.connection, &delivered)?),
            ["01a4", "01a1"]
        );
        Ok(())
    }

    #[test]
    fn filters_by_role() -> Result<()> {
        let fixture = populated()?;
        let buyer = InvoiceFilter {
            role: Some(Role::Buyer),
            ..filter()
        };

        assert_eq!(ids(&find(&fixture.connection, &buyer)?), ["01a2"]);
        Ok(())
    }

    #[test]
    fn combines_direction_state_and_status_filters() -> Result<()> {
        let fixture = populated()?;
        let rejected = InvoiceFilter {
            directions: vec![Direction::Inbound],
            states: vec!["DELIVERED".to_owned(), "INVALID".to_owned()],
            statuses: vec!["REJECTED".to_owned()],
            ..filter()
        };

        assert_eq!(ids(&find(&fixture.connection, &rejected)?), ["01a1"]);
        Ok(())
    }

    #[test]
    fn filters_by_creation_period() -> Result<()> {
        let fixture = Fixture::new()?;
        let bound = |instant: &str| UuidBound::try_from(instant.parse::<jiff::Timestamp>()?);
        fixture.insert(
            "01a069b7-1000-7000-8000-000000000001",
            "INBOUND",
            IDENTIFIER,
            OTHER,
            "VALID",
            None,
        )?;
        fixture.insert(
            "01a06edd-6c00-7000-8000-000000000002",
            "INBOUND",
            IDENTIFIER,
            OTHER,
            "VALID",
            None,
        )?;
        let first_day = InvoiceFilter {
            from: Some(bound("2026-09-04T00:00:00Z")?),
            to: Some(bound("2026-09-05T00:00:00Z")?),
            ..filter()
        };

        assert_eq!(
            ids(&find(&fixture.connection, &first_day)?),
            ["01a069b7-1000-7000-8000-000000000001"]
        );
        Ok(())
    }

    #[test]
    fn limit_keeps_the_total() -> Result<()> {
        let fixture = populated()?;
        let first = InvoiceFilter {
            limit: 1,
            ..filter()
        };

        let page = find(&fixture.connection, &first)?;

        assert_eq!(ids(&page), ["01a3"]);
        assert_eq!(page.total, 3);
        Ok(())
    }

    #[test]
    fn unknown_identifier_finds_nothing() -> Result<()> {
        let fixture = populated()?;
        let unknown = InvoiceFilter {
            identifier_value: Some("000000000".to_owned()),
            ..filter()
        };

        assert_eq!(find(&fixture.connection, &unknown)?, Page::default());
        Ok(())
    }

    #[test]
    fn identifiers_of_any_scheme_are_searchable() -> Result<()> {
        let fixture = populated()?;
        fixture.insert("01a5", "INBOUND", "0123456789", OTHER, "VALID", None)?;
        let bce = InvoiceFilter {
            identifier_value: Some("0123456789".to_owned()),
            ..filter()
        };

        assert_eq!(ids(&find(&fixture.connection, &bce)?), ["01a5"]);
        Ok(())
    }

    #[test]
    fn counts_invoices_by_state_and_last_status() -> Result<()> {
        let fixture = populated()?;

        let mut counts = count_by_state(
            &fixture.connection,
            &[Direction::Inbound, Direction::Outbound],
        )?;
        counts
            .sort_by(|a, b| (&a.state, &a.last_status_code).cmp(&(&b.state, &b.last_status_code)));

        let count = |state: &str, status: Option<&str>, count| StateCount {
            state: state.to_owned(),
            last_status_code: status.map(str::to_owned),
            count,
        };
        assert_eq!(
            counts,
            [
                count("DELIVERED", None, 1),
                count("DELIVERED", Some("REJECTED"), 1),
                count("INVALID", None, 1),
                count("VALID", Some("APPROVED"), 1),
            ]
        );
        Ok(())
    }

    #[test]
    fn counts_only_the_invoices_of_the_directions() -> Result<()> {
        let fixture = populated()?;

        let outbound = count_by_state(&fixture.connection, &[Direction::Outbound])?;

        assert_eq!(
            outbound,
            [StateCount {
                state: "VALID".to_owned(),
                last_status_code: Some("APPROVED".to_owned()),
                count: 1,
            }]
        );
        Ok(())
    }

    #[test]
    fn reads_the_parties_of_a_synchronized_invoice() -> Result<()> {
        let fixture = Fixture::new()?;
        InvoiceRecord {
            id: "01a2",
            seller_name: Some("Leroy Merlin France"),
            seller_identifier: Some("384560942"),
            buyer_name: Some("BVS RENOVATION"),
            ..InvoiceRecord::default()
        }
        .insert(&fixture.connection)?;

        let parties = parties(&fixture.connection, "01a2")?;

        assert_eq!(
            parties,
            Some(InvoiceParties {
                seller_name: Some("Leroy Merlin France".to_owned()),
                seller_identifier: Some("384560942".to_owned()),
                buyer_name: Some("BVS RENOVATION".to_owned()),
                buyer_identifier: None,
            })
        );
        assert_eq!(super::parties(&fixture.connection, "01a3")?, None);
        Ok(())
    }
}
