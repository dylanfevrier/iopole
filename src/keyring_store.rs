use std::cell::RefCell;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::Result;
use crate::credentials::{CredentialStore, Credentials};
use crate::database::DatabaseKeyStore;

const SERVICE: &str = "iopole";
const ACCOUNT: &str = "secrets";

/// All secrets share a single keychain entry, so that macOS asks for access at
/// most once.
#[derive(Default, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
struct Secrets {
    client_id: Option<String>,
    client_secret: Option<String>,
    customer_id: Option<String>,
    database_key: Option<String>,
}

impl Secrets {
    fn is_empty(&self) -> bool {
        self.client_secret.is_none() && self.database_key.is_none()
    }
}

#[derive(Default)]
pub(crate) struct KeyringStore {
    cache: RefCell<Option<Secrets>>,
}

impl KeyringStore {
    fn entry() -> Result<keyring::Entry> {
        Ok(keyring::Entry::new(SERVICE, ACCOUNT)?)
    }

    fn secrets(&self) -> Result<Secrets> {
        if let Some(secrets) = self.cache.borrow().as_ref() {
            return Ok(secrets.clone());
        }
        let secrets = match Self::entry()?.get_password() {
            Ok(json) => serde_json::from_str(SecretString::from(json).expose_secret())?,
            Err(keyring::Error::NoEntry) => Secrets::default(),
            Err(err) => return Err(err.into()),
        };
        *self.cache.borrow_mut() = Some(secrets.clone());
        Ok(secrets)
    }

    fn update(&self, change: impl FnOnce(&mut Secrets)) -> Result<()> {
        let mut secrets = self.secrets()?;
        change(&mut secrets);
        if secrets.is_empty() {
            match Self::entry()?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(err) => return Err(err.into()),
            }
        } else {
            let json = SecretString::from(serde_json::to_string(&secrets)?);
            Self::entry()?.set_password(json.expose_secret())?;
        }
        *self.cache.borrow_mut() = Some(secrets);
        Ok(())
    }
}

impl CredentialStore for KeyringStore {
    fn has_credentials(&self) -> Result<bool> {
        Ok(self.secrets()?.client_secret.is_some())
    }

    fn load(&self) -> Result<Option<Credentials>> {
        let secrets = self.secrets()?;
        let (Some(client_id), Some(client_secret), Some(customer_id)) = (
            &secrets.client_id,
            &secrets.client_secret,
            &secrets.customer_id,
        ) else {
            return Ok(None);
        };
        Ok(Some(Credentials {
            client_id: client_id.clone(),
            client_secret: SecretString::from(client_secret.as_str()),
            customer_id: customer_id.clone(),
        }))
    }

    fn save(&self, credentials: &Credentials) -> Result<()> {
        self.update(|secrets| {
            secrets.client_id = Some(credentials.client_id.clone());
            secrets.client_secret = Some(credentials.client_secret.expose_secret().to_owned());
            secrets.customer_id = Some(credentials.customer_id.clone());
        })
    }

    fn delete(&self) -> Result<()> {
        self.update(|secrets| {
            secrets.client_id.zeroize();
            secrets.client_secret.zeroize();
            secrets.customer_id.zeroize();
        })
    }
}

impl DatabaseKeyStore for KeyringStore {
    fn load_key(&self) -> Result<Option<SecretString>> {
        Ok(self
            .secrets()?
            .database_key
            .as_deref()
            .map(SecretString::from))
    }

    fn save_key(&self, key: &SecretString) -> Result<()> {
        self.update(|secrets| secrets.database_key = Some(key.expose_secret().to_owned()))
    }

    fn delete_key(&self) -> Result<()> {
        self.update(|secrets| secrets.database_key.zeroize())
    }
}
