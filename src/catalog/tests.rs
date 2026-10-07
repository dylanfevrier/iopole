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
        identifier_value: IDENTIFIER.to_owned(),
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
        identifier_value: "000000000".to_owned(),
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
        identifier_value: "0123456789".to_owned(),
        ..filter()
    };

    assert_eq!(ids(&find(&fixture.connection, &bce)?), ["01a5"]);
    Ok(())
}
