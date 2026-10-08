use serde_json::{Value, json};

use crate::invoice::Direction;

const COMPANY_UID_HEADER: &str = "X-Company-UID";

/// How the operator claims a business entity: Iopole replaces the whole
/// configuration with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Claim {
    /// Identifier of the company in the operator's own system, sent back in
    /// the webhooks.
    pub(crate) company_uid: String,
    /// `None` for both directions.
    pub(crate) direction: Option<Direction>,
}

impl Claim {
    pub(crate) fn body(&self) -> Value {
        let mut body = json!({
            "data": {
                "header": [{ "key": COMPANY_UID_HEADER, "value": self.company_uid }]
            }
        });
        if let Some(direction) = self.direction {
            body["direction"] = direction.as_str().into();
        }
        body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_the_company_uid_as_a_webhook_header_in_both_directions() {
        let claim = Claim {
            company_uid: "6071797".to_owned(),
            direction: None,
        };

        assert_eq!(
            claim.body(),
            json!({ "data": { "header": [{ "key": "X-Company-UID", "value": "6071797" }] } })
        );
    }

    #[test]
    fn restricts_the_direction_when_given() {
        let claim = Claim {
            company_uid: "6071797".to_owned(),
            direction: Some(Direction::Inbound),
        };

        assert_eq!(claim.body()["direction"], "INBOUND");
    }
}
