use std::cell::RefCell;

use super::*;

#[derive(Default)]
pub(crate) struct InMemoryKeyStore {
    key: RefCell<Option<SecretString>>,
}

impl DatabaseKeyStore for InMemoryKeyStore {
    fn load_key(&self) -> Result<Option<SecretString>> {
        Ok(self.key.borrow().clone())
    }

    fn save_key(&self, key: &SecretString) -> Result<()> {
        *self.key.borrow_mut() = Some(key.clone());
        Ok(())
    }

    fn delete_key(&self) -> Result<()> {
        self.key.borrow_mut().take();
        Ok(())
    }
}

pub(crate) struct InvoiceRecord<'a> {
    pub(crate) id: &'a str,
    pub(crate) direction: &'a str,
    pub(crate) created_at: &'a str,
    pub(crate) number: Option<&'a str>,
    pub(crate) amount: Option<&'a str>,
    pub(crate) seller_name: Option<&'a str>,
    pub(crate) seller_identifier: Option<&'a str>,
    pub(crate) buyer_name: Option<&'a str>,
    pub(crate) buyer_identifier: Option<&'a str>,
    pub(crate) state: &'a str,
    pub(crate) last_status_code: Option<&'a str>,
}

impl Default for InvoiceRecord<'_> {
    fn default() -> Self {
        Self {
            id: "01a1",
            direction: "INBOUND",
            created_at: "2026-09-01T10:00:00.000+00:00",
            number: None,
            amount: None,
            seller_name: None,
            seller_identifier: None,
            buyer_name: None,
            buyer_identifier: None,
            state: "VALID",
            last_status_code: None,
        }
    }
}

impl InvoiceRecord<'_> {
    pub(crate) fn insert(&self, connection: &rusqlite::Connection) -> Result<()> {
        connection.execute(
            "INSERT INTO invoice (invoice_id, direction, created_at, number, amount,
                                  seller_name, seller_identifier, buyer_name, buyer_identifier,
                                  state, last_status_code, raw, synced_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, '{}', '')",
            rusqlite::params![
                self.id,
                self.direction,
                self.created_at,
                self.number,
                self.amount,
                self.seller_name,
                self.seller_identifier,
                self.buyer_name,
                self.buyer_identifier,
                self.state,
                self.last_status_code,
            ],
        )?;
        Ok(())
    }
}
