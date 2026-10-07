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
mod tests;
