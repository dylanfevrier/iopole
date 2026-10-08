use std::io::Write;

use jiff::Timestamp;

use crate::Result;
use crate::catalog::InvoiceParties;
use crate::iopole::{StatusEvent, StatusParty, buyer, seller};
use crate::render::{Fields, MISSING, Table};

pub(super) fn history(
    out: &mut impl Write,
    invoice_id: &str,
    synchronized: Option<&InvoiceParties>,
    mut history: Vec<StatusEvent>,
) -> Result<()> {
    let local = synchronized.cloned().unwrap_or_default();
    let mut fields = Fields::default();
    fields.push("ID", invoice_id);
    fields.push(
        "Seller",
        party(local.seller_name, local.seller_identifier, seller(&history)),
    );
    fields.push(
        "Buyer",
        party(local.buyer_name, local.buyer_identifier, buyer(&history)),
    );
    writeln!(out, "{fields}")?;

    if history.is_empty() {
        writeln!(out, "No lifecycle status exchanged on the network yet.")?;
        return Ok(());
    }
    history.sort_by_key(|event| std::cmp::Reverse(instant(event)));
    let mut table = Table::new(&["DATE (UTC)", "STATUS"]);
    for event in &history {
        table.push([date(event), status(event)]);
    }
    write!(out, "{table}")?;
    Ok(())
}

pub(super) fn unknown_invoice(out: &mut impl Write, invoice_id: &str) -> Result<()> {
    writeln!(out, "Iopole knows no invoice {invoice_id}.")?;
    Ok(())
}

/// The synchronized name, then the SIREN the statuses carry.
fn party(
    synchronized_name: Option<String>,
    synchronized_identifier: Option<String>,
    exchanged: Option<&StatusParty>,
) -> String {
    let name = synchronized_name
        .or_else(|| exchanged?.name.clone())
        .unwrap_or_else(|| MISSING.to_owned());
    match exchanged.and_then(|party| party.siren.as_deref()) {
        Some(siren) => format!("{name} (SIREN {siren})"),
        None => match synchronized_identifier {
            Some(identifier) => format!("{name} ({identifier})"),
            None => name,
        },
    }
}

fn instant(event: &StatusEvent) -> Option<Timestamp> {
    event.date.as_deref()?.parse().ok()
}

fn date(event: &StatusEvent) -> String {
    match instant(event) {
        Some(instant) => instant.strftime("%Y-%m-%d %H:%M:%S").to_string(),
        None => event.date.clone().unwrap_or_else(|| MISSING.to_owned()),
    }
}

fn status(event: &StatusEvent) -> String {
    let Some(status) = &event.status else {
        return MISSING.to_owned();
    };
    let code = status.code.as_deref().unwrap_or(MISSING);
    match &status.network_code {
        Some(network_code) => format!("{code} ({network_code})"),
        None => code.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::iopole::testing::{approved, payment_received};

    const INVOICE_ID: &str = "01a10b05-e3fd-74c8-b9cd-ce8e4c012777";

    fn rendered(synchronized: Option<&InvoiceParties>, events: Vec<StatusEvent>) -> String {
        let mut out = Vec::new();
        history(&mut out, INVOICE_ID, synchronized, events)
            .unwrap_or_else(|err| panic!("cannot render: {err}"));
        String::from_utf8_lossy(&out).into_owned()
    }

    #[test]
    fn shows_the_parties_then_the_statuses_most_recent_first() {
        let synchronized = InvoiceParties {
            seller_name: Some("Leroy Merlin France".to_owned()),
            buyer_name: Some("BVS RENOVATION".to_owned()),
            ..InvoiceParties::default()
        };

        assert_eq!(
            rendered(Some(&synchronized), vec![approved(), payment_received()]),
            "\
ID:           01a10b05-e3fd-74c8-b9cd-ce8e4c012777
Seller:       Leroy Merlin France (SIREN 384560942)
Buyer:        BVS RENOVATION (SIREN 980217632)

DATE (UTC)           STATUS
2026-10-05 09:45:07  PAYMENT_RECEIVED (212)
2026-10-01 14:02:11  APPROVED (205)
"
        );
    }

    #[test]
    fn tells_when_no_status_was_exchanged() {
        let synchronized = InvoiceParties {
            seller_name: Some("CASTORAMA Dijon".to_owned()),
            seller_identifier: Some("451678973".to_owned()),
            ..InvoiceParties::default()
        };

        assert_eq!(
            rendered(Some(&synchronized), Vec::new()),
            "\
ID:           01a10b05-e3fd-74c8-b9cd-ce8e4c012777
Seller:       CASTORAMA Dijon (451678973)
Buyer:        -

No lifecycle status exchanged on the network yet.
"
        );
    }
}
