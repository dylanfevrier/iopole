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
