use jiff::Timestamp;

use super::*;
use crate::Result;

fn bound(instant: &str) -> Result<UuidBound> {
    UuidBound::try_from(instant.parse::<Timestamp>()?)
}

#[test]
fn renders_documented_query() -> Result<()> {
    let query = SearchQuery {
        direction: Direction::Inbound,
        from: bound("2026-09-04T00:00:00Z")?,
        to: bound("2026-09-05T00:00:00Z")?,
    };

    assert_eq!(
        query.to_string(),
        r#"invoice.direction:"INBOUND" AND invoice.invoiceId:>="01a069b7-1000-0000-0000-000000000000" AND invoice.invoiceId:<"01a06edd-6c00-0000-0000-000000000000""#
    );
    Ok(())
}
