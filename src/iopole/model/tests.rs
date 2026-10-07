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
