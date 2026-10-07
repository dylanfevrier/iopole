use secrecy::{ExposeSecret, SecretString};

use crate::Result;

const KEYRING_SERVICE: &str = "iopole";
const KEYRING_CLIENT_ID: &str = "client-id";
const KEYRING_CLIENT_SECRET: &str = "client-secret";
const KEYRING_CUSTOMER_ID: &str = "customer-id";

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

pub(crate) struct KeyringStore;

impl KeyringStore {
    fn entry(name: &str) -> Result<keyring::Entry> {
        Ok(keyring::Entry::new(KEYRING_SERVICE, name)?)
    }
}

impl CredentialStore for KeyringStore {
    fn has_credentials(&self) -> Result<bool> {
        match Self::entry(KEYRING_CLIENT_SECRET)?.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(err) => Err(err.into()),
        }
    }

    fn save(&self, credentials: &Credentials) -> Result<()> {
        Self::entry(KEYRING_CLIENT_ID)?.set_password(&credentials.client_id)?;
        Self::entry(KEYRING_CLIENT_SECRET)?
            .set_password(credentials.client_secret.expose_secret())?;
        Self::entry(KEYRING_CUSTOMER_ID)?.set_password(&credentials.customer_id)?;
        Ok(())
    }

    fn delete(&self) -> Result<()> {
        for name in [
            KEYRING_CLIENT_ID,
            KEYRING_CLIENT_SECRET,
            KEYRING_CUSTOMER_ID,
        ] {
            match Self::entry(name)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(err) => return Err(err.into()),
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use std::cell::RefCell;

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
