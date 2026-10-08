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

pub(super) fn invoices(out: &mut impl Write, page: &Page) -> Result<()> {
    let mut table = Table::new(&COLUMNS).align_right("AMOUNT");
    for invoice in &page.invoices {
        table.push(cells(invoice));
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

pub(super) fn no_invoice(out: &mut impl Write, sync_command: &str) -> Result<()> {
    writeln!(out, "No invoice found.")?;
    writeln!(
        out,
        "Run `{sync_command}` to retrieve invoices from Iopole, then search again."
    )?;
    Ok(())
}

fn cells(invoice: &InvoiceRow) -> [String; 9] {
    let optional = |value: &Option<String>| value.clone().unwrap_or_else(|| MISSING.to_owned());
    [
        invoice.id.clone(),
        date(&invoice.created_at),
        invoice.direction.to_lowercase(),
        optional(&invoice.number),
        truncate(&optional(&invoice.seller_name)),
        truncate(&optional(&invoice.buyer_name)),
        optional(&invoice.amount),
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
        let mut out = Vec::new();
        invoices(&mut out, page).unwrap_or_else(|err| panic!("cannot render: {err}"));
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
            buyer_name: Some("LES RAVALEURS MANTOIS ET ASSOCIES DE LA VALLEE".to_owned()),
            state: "DELIVERED".to_owned(),
            last_status_code: Some("APPROVED".to_owned()),
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
            buyer_name: Some("CCLS".to_owned()),
            state: "PENDING".to_owned(),
            last_status_code: None,
        }
    }

    #[test]
    fn prints_invoices_as_a_table() {
        let page = Page {
            invoices: vec![delivered(), pending()],
            total: 2,
        };

        assert_eq!(
            rendered(&page),
            "\
ID                                    DATE        DIRECTION  NUMBER       SELLER  BUYER                            AMOUNT  STATE      STATUS
0199a8f2-6b1c-7d3e-9f40-1a2b3c4d5e6f  2026-09-02  inbound    F-2026-0042  CCLS    LES RAVALEURS MANTOIS ET ASSO…  1250.00  DELIVERED  APPROVED
0199a3c1-0e5d-7a21-8b77-6c5d4e3f2a10  2026-09-01  outbound   -            -       CCLS                                 60  PENDING    -

2 invoices found.
"
        );
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
