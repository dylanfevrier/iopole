use secrecy::SecretString;

use crate::Result;

#[derive(Debug)]
pub(crate) struct Credentials {
    pub(crate) client_id: String,
    pub(crate) client_secret: SecretString,
    pub(crate) customer_id: String,
}

pub(crate) trait CredentialStore {
    fn has_credentials(&self) -> Result<bool>;
    fn save(&self, credentials: &Credentials) -> Result<()>;
    fn delete(&self) -> Result<()>;
}

#[cfg(test)]
pub(crate) mod testing {
    use std::cell::RefCell;

    use secrecy::ExposeSecret;

    use super::*;

    #[derive(Default)]
    pub(crate) struct InMemoryStore {
        pub(crate) saved: RefCell<Option<(String, String, String)>>,
    }

    impl InMemoryStore {
        pub(crate) fn logged_in() -> Self {
            let store = Self::default();
            *store.saved.borrow_mut() = Some((
                "existing-client".to_owned(),
                "existing".to_owned(),
                "existing-customer".to_owned(),
            ));
            store
        }
    }

    impl CredentialStore for InMemoryStore {
        fn has_credentials(&self) -> Result<bool> {
            Ok(self.saved.borrow().is_some())
        }

        fn save(&self, credentials: &Credentials) -> Result<()> {
            *self.saved.borrow_mut() = Some((
                credentials.client_id.clone(),
                credentials.client_secret.expose_secret().to_owned(),
                credentials.customer_id.clone(),
            ));
            Ok(())
        }

        fn delete(&self) -> Result<()> {
            self.saved.borrow_mut().take();
            Ok(())
        }
    }
}
