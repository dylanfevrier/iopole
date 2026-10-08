use serde::Deserialize;
use serde_json::Value;

use crate::Result;
use crate::invoice::{Invoice, Party, Status};

#[derive(Deserialize)]
struct SearchResponse {
    data: Vec<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InvoiceDto {
    metadata: MetadataDto,
    last_status_data: Option<LastStatusDto>,
    business_data: Option<BusinessDataDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetadataDto {
    invoice_id: String,
    state: String,
    direction: String,
    create_date: String,
}

#[derive(Deserialize)]
struct LastStatusDto {
    code: Option<String>,
    state: Option<String>,
    date: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BusinessDataDto {
    invoice_id: Option<String>,
    monetary: Option<MonetaryDto>,
    seller: Option<PartyDto>,
    buyer: Option<PartyDto>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MonetaryDto {
    invoice_amount: Option<AmountDto>,
}

#[derive(Deserialize)]
struct AmountDto {
    /// Documented as a string, accepted as a number too.
    amount: Option<Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PartyDto {
    name: Option<String>,
    electronic_address: Option<String>,
    country: Option<String>,
}

pub(crate) fn parse_page(body: &str) -> Result<Vec<Invoice>> {
    let response: SearchResponse = serde_json::from_str(body)?;
    response.data.into_iter().map(parse_invoice).collect()
}

fn parse_invoice(mut item: Value) -> Result<Invoice> {
    remove_payment_means(&mut item);
    let dto = InvoiceDto::deserialize(&item)?;
    let business = dto.business_data;
    let last_status = dto.last_status_data;

    Ok(Invoice {
        id: dto.metadata.invoice_id,
        direction: dto.metadata.direction,
        created_at: dto.metadata.create_date,
        status: Status {
            state: dto.metadata.state,
            last_code: last_status.as_ref().and_then(|s| s.code.clone()),
            last_state: last_status.as_ref().and_then(|s| s.state.clone()),
            last_date: last_status.and_then(|s| s.date),
        },
        number: business.as_ref().and_then(|b| b.invoice_id.clone()),
        amount: business
            .as_ref()
            .and_then(|b| {
                b.monetary
                    .as_ref()?
                    .invoice_amount
                    .as_ref()?
                    .amount
                    .as_ref()
            })
            .map(amount_to_string),
        seller: business
            .as_ref()
            .and_then(|b| b.seller.as_ref())
            .map(Party::from)
            .unwrap_or_default(),
        buyer: business
            .as_ref()
            .and_then(|b| b.buyer.as_ref())
            .map(Party::from)
            .unwrap_or_default(),
        raw: item.to_string(),
    })
}

/// Payment means hold IBANs and must never reach the database.
fn remove_payment_means(item: &mut Value) {
    if let Some(business) = item.get_mut("businessData").and_then(Value::as_object_mut) {
        business.remove("paymentMeans");
    }
}

fn amount_to_string(amount: &Value) -> String {
    match amount {
        Value::String(amount) => amount.clone(),
        other => other.to_string(),
    }
}

impl From<&PartyDto> for Party {
    fn from(dto: &PartyDto) -> Self {
        Self {
            name: dto.name.clone(),
            electronic_address: dto.electronic_address.clone(),
            country: dto.country.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOCUMENTED_PAGE: &str = r#"{
  "data": [
    {
      "metadata": {
        "invoiceId": "01a117de-89d0-7300-bbd3-8ff310c409b4",
        "state": "INVALID",
        "direction": "INBOUND",
        "createDate": "2026-10-07T19:37:02.672+00:00"
      },
      "lastStatusData": {
        "code": "REJECTED",
        "date": "2026-10-07T19:37:02.71+00:00",
        "state": "DELIVERED"
      },
      "businessData": {
        "invoiceId": "004791000-0000219237",
        "monetary": { "invoiceAmount": { "amount": "60" } },
        "seller": { "name": "CCLS", "country": "FR", "electronicAddress": "0225:352862346" },
        "buyer": { "name": "LES RAVALEURS MANTOIS", "country": "FR", "electronicAddress": "0225:517657334" },
        "paymentMeans": [ { "typeCode": "30", "bankTransfer": { "iban": "FR7600000000000000000000000", "bic": "XXXXFRPP" } } ]
      }
    }
  ],
  "meta": { "offset": 0, "limit": 200, "count": 1 }
}"#;

    #[test]
    fn parses_documented_payload() -> Result<()> {
        let invoices = parse_page(DOCUMENTED_PAGE)?;

        let [invoice] = invoices.as_slice() else {
            panic!("expected one invoice, got {}", invoices.len());
        };
        assert_eq!(invoice.id, "01a117de-89d0-7300-bbd3-8ff310c409b4");
        assert_eq!(invoice.direction, "INBOUND");
        assert_eq!(invoice.created_at, "2026-10-07T19:37:02.672+00:00");
        assert_eq!(
            invoice.status,
            Status {
                state: "INVALID".to_owned(),
                last_code: Some("REJECTED".to_owned()),
                last_state: Some("DELIVERED".to_owned()),
                last_date: Some("2026-10-07T19:37:02.71+00:00".to_owned()),
            }
        );
        assert_eq!(invoice.number.as_deref(), Some("004791000-0000219237"));
        assert_eq!(invoice.amount.as_deref(), Some("60"));
        assert_eq!(invoice.seller.name.as_deref(), Some("CCLS"));
        assert_eq!(invoice.buyer.identifier_value(), Some("517657334"));
        Ok(())
    }

    #[test]
    fn raw_json_never_contains_payment_means() -> Result<()> {
        let invoices = parse_page(DOCUMENTED_PAGE)?;

        for invoice in invoices {
            assert!(!invoice.raw.contains("paymentMeans"));
            assert!(!invoice.raw.contains("FR7600000000000000000000000"));
            assert!(invoice.raw.contains("004791000-0000219237"));
        }
        Ok(())
    }

    #[test]
    fn only_metadata_is_required() -> Result<()> {
        let body = r#"{"data":[{"metadata":{"invoiceId":"01a1","state":"PENDING","direction":"OUTBOUND","createDate":"2026-10-07T00:00:00Z"}}]}"#;

        let invoices = parse_page(body)?;

        assert_eq!(invoices.len(), 1);
        assert_eq!(invoices[0].status.last_date, None);
        assert_eq!(invoices[0].seller, Party::default());
        Ok(())
    }

    #[test]
    fn accepts_numeric_amounts() -> Result<()> {
        let body = r#"{"data":[{"metadata":{"invoiceId":"01a1","state":"VALID","direction":"INBOUND","createDate":"2026-10-07T00:00:00Z"},"businessData":{"monetary":{"invoiceAmount":{"amount":12.5}}}}]}"#;

        assert_eq!(parse_page(body)?[0].amount.as_deref(), Some("12.5"));
        Ok(())
    }

    #[test]
    fn rejects_results_without_metadata() {
        assert!(parse_page(r#"{"data":[{"businessData":{}}]}"#).is_err());
    }
}
