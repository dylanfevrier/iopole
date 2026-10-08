use std::io::Write;

use jiff::Timestamp;

use crate::Result;
use crate::catalog::{InvoiceRow, Page};
use crate::render::{MISSING, Table};

const NAME_WIDTH: usize = 30;
const COLUMNS: [&str; 9] = [
    "ID",
    "DATE",
    "DIRECTION",
    "NUMBER",
    "SELLER",
    "BUYER",
    "AMOUNT",
    "STATE",
    "STATUS",
];

pub(super) fn invoices(out: &mut impl Write, page: &Page, identifiers: bool) -> Result<()> {
    let mut table = Table::new(&COLUMNS).align_right("AMOUNT");
    for invoice in &page.invoices {
        table.push(cells(invoice, identifiers));
    }
    write!(out, "{table}")?;

    if page.invoices.len() < page.total {
        writeln!(
            out,
            "\nShowing {} of {} invoices; use --limit to see more.",
            page.invoices.len(),
            page.total
        )?;
    } else {
        writeln!(out, "\n{} invoices found.", page.total)?;
    }
    Ok(())
}

/// The invoices as Iopole returned them when synchronized, pretty-printed.
pub(super) fn json(out: &mut impl Write, page: &Page) -> Result<()> {
    let invoices = page
        .invoices
        .iter()
        .map(|invoice| serde_json::from_str(&invoice.raw))
        .collect::<std::result::Result<Vec<serde_json::Value>, _>>()?;
    serde_json::to_writer_pretty(&mut *out, &invoices)?;
    writeln!(out)?;
    Ok(())
}

pub(super) fn no_invoice(out: &mut impl Write, sync_command: &str) -> Result<()> {
    writeln!(out, "No invoice found.")?;
    writeln!(
        out,
        "Run `{sync_command}` to retrieve invoices from Iopole, then search again."
    )?;
    Ok(())
}

fn cells(invoice: &InvoiceRow, identifiers: bool) -> [String; 9] {
    let optional = |value: &Option<String>| value.clone().unwrap_or_else(|| MISSING.to_owned());
    [
        invoice.id.clone(),
        date(&invoice.created_at),
        invoice.direction.to_lowercase(),
        optional(&invoice.number),
        party(
            invoice.seller_identifier.as_deref().filter(|_| identifiers),
            invoice.seller_name.as_deref(),
        ),
        party(
            invoice.buyer_identifier.as_deref().filter(|_| identifiers),
            invoice.buyer_name.as_deref(),
        ),
        invoice
            .amount
            .as_deref()
            .map_or_else(|| MISSING.to_owned(), amount),
        invoice.state.clone(),
        optional(&invoice.last_status_code),
    ]
}

fn date(created_at: &str) -> String {
    created_at.parse::<Timestamp>().map_or_else(
        |_| created_at.chars().take(10).collect(),
        |instant| instant.strftime("%Y-%m-%d").to_string(),
    )
}

/// Always two decimals after a comma (`1250.5` reads `1250,50`), rounded half
/// away from zero; an amount that is not a decimal number shows as received.
fn amount(value: &str) -> String {
    cents(value).map_or_else(
        || value.to_owned(),
        |(negative, cents)| {
            let sign = if negative { "-" } else { "" };
            format!("{sign}{},{:02}", cents / 100, cents % 100)
        },
    )
}

fn cents(value: &str) -> Option<(bool, u128)> {
    let value = value.trim();
    let (negative, unsigned) = match value.strip_prefix('-') {
        Some(unsigned) => (true, unsigned),
        None => (false, value.strip_prefix('+').unwrap_or(value)),
    };
    let (units, decimals) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let digits = |part: &str| part.chars().all(|c| c.is_ascii_digit());
    if units.is_empty() || !digits(units) || !digits(decimals) {
        return None;
    }
    let mut decimals = decimals.bytes().map(|digit| u128::from(digit - b'0'));
    let tenths = decimals.next().unwrap_or(0);
    let hundredths = decimals.next().unwrap_or(0);
    let round_up = decimals.next().is_some_and(|digit| digit >= 5);
    let cents = units
        .parse::<u128>()
        .ok()?
        .checked_mul(100)?
        .checked_add(tenths * 10 + hundredths + u128::from(round_up))?;
    Some((negative && cents > 0, cents))
}

/// `identifier - name`, so that the identifier can be passed to
/// `entity search`; only the name is truncated.
fn party(identifier: Option<&str>, name: Option<&str>) -> String {
    match (identifier, name) {
        (Some(identifier), Some(name)) => format!("{identifier} - {}", truncate(name)),
        (Some(identifier), None) => identifier.to_owned(),
        (None, Some(name)) => truncate(name),
        (None, None) => MISSING.to_owned(),
    }
}

fn truncate(name: &str) -> String {
    if name.chars().count() <= NAME_WIDTH {
        name.to_owned()
    } else {
        let kept: String = name.chars().take(NAME_WIDTH - 1).collect();
        format!("{kept}…")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(page: &Page) -> String {
        rendered_with(page, false)
    }

    fn rendered_with(page: &Page, identifiers: bool) -> String {
        let mut out = Vec::new();
        invoices(&mut out, page, identifiers).unwrap_or_else(|err| panic!("cannot render: {err}"));
        String::from_utf8_lossy(&out).into_owned()
    }

    fn delivered() -> InvoiceRow {
        InvoiceRow {
            id: "0199a8f2-6b1c-7d3e-9f40-1a2b3c4d5e6f".to_owned(),
            direction: "INBOUND".to_owned(),
            created_at: "2026-09-02T23:30:00.000+00:00".to_owned(),
            number: Some("F-2026-0042".to_owned()),
            amount: Some("1250.00".to_owned()),
            seller_name: Some("CCLS".to_owned()),
            seller_identifier: Some("352862346".to_owned()),
            buyer_name: Some("LES RAVALEURS MANTOIS ET ASSOCIES DE LA VALLEE".to_owned()),
            buyer_identifier: Some("517657334".to_owned()),
            state: "DELIVERED".to_owned(),
            last_status_code: Some("APPROVED".to_owned()),
            raw: "{}".to_owned(),
        }
    }

    fn pending() -> InvoiceRow {
        InvoiceRow {
            id: "0199a3c1-0e5d-7a21-8b77-6c5d4e3f2a10".to_owned(),
            direction: "OUTBOUND".to_owned(),
            created_at: "2026-09-01T08:00:00.000+00:00".to_owned(),
            number: None,
            amount: Some("60".to_owned()),
            seller_name: None,
            seller_identifier: None,
            buyer_name: Some("CCLS".to_owned()),
            buyer_identifier: Some("352862346".to_owned()),
            state: "PENDING".to_owned(),
            last_status_code: None,
            raw: "{}".to_owned(),
        }
    }

    #[test]
    fn prints_invoices_as_a_table_with_names_only() {
        let page = Page {
            invoices: vec![delivered(), pending()],
            total: 2,
        };

        assert_eq!(
            rendered(&page),
            "\
ID                                    DATE        DIRECTION  NUMBER       SELLER  BUYER                            AMOUNT  STATE      STATUS
0199a8f2-6b1c-7d3e-9f40-1a2b3c4d5e6f  2026-09-02  inbound    F-2026-0042  CCLS    LES RAVALEURS MANTOIS ET ASSO…  1250,00  DELIVERED  APPROVED
0199a3c1-0e5d-7a21-8b77-6c5d4e3f2a10  2026-09-01  outbound   -            -       CCLS                              60,00  PENDING    -

2 invoices found.
"
        );
    }

    #[test]
    fn shows_identifiers_before_names_on_demand() {
        let page = Page {
            invoices: vec![delivered(), pending()],
            total: 2,
        };

        assert_eq!(
            rendered_with(&page, true),
            "\
ID                                    DATE        DIRECTION  NUMBER       SELLER            BUYER                                        AMOUNT  STATE      STATUS
0199a8f2-6b1c-7d3e-9f40-1a2b3c4d5e6f  2026-09-02  inbound    F-2026-0042  352862346 - CCLS  517657334 - LES RAVALEURS MANTOIS ET ASSO…  1250,00  DELIVERED  APPROVED
0199a3c1-0e5d-7a21-8b77-6c5d4e3f2a10  2026-09-01  outbound   -            -                 352862346 - CCLS                              60,00  PENDING    -

2 invoices found.
"
        );
    }

    #[test]
    fn parties_show_whatever_is_known() {
        assert_eq!(party(Some("0123456789"), None), "0123456789");
        assert_eq!(party(None, Some("CCLS")), "CCLS");
        assert_eq!(party(None, None), "-");
    }

    #[test]
    fn amounts_have_two_decimals_after_a_comma() {
        for (received, shown) in [
            ("60", "60,00"),
            ("12.5", "12,50"),
            ("1250.00", "1250,00"),
            ("0.07", "0,07"),
            ("-3.4", "-3,40"),
            ("19.995", "20,00"),
            ("19.994", "19,99"),
            ("-0.001", "0,00"),
            ("n/a", "n/a"),
            ("12,5", "12,5"),
        ] {
            assert_eq!(amount(received), shown, "for {received}");
        }
    }

    #[test]
    fn tells_when_results_are_truncated() {
        let page = Page {
            invoices: vec![delivered()],
            total: 2,
        };

        assert!(rendered(&page).ends_with("\nShowing 1 of 2 invoices; use --limit to see more.\n"));
    }

    #[test]
    fn invites_to_sync() -> Result<()> {
        let mut out = Vec::new();

        no_invoice(&mut out, "iopole document sync --from <date>")?;

        assert_eq!(
            String::from_utf8_lossy(&out),
            "No invoice found.\nRun `iopole document sync --from <date>` to retrieve invoices from Iopole, then search again.\n"
        );
        Ok(())
    }
}
