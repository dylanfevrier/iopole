use secrecy::{ExposeSecret, SecretString};

use crate::Result;
use crate::credentials::{CredentialStore, Credentials};
use crate::database::DatabaseKeyStore;

const SERVICE: &str = "iopole";
const CLIENT_ID: &str = "client-id";
const CLIENT_SECRET: &str = "client-secret";
const CUSTOMER_ID: &str = "customer-id";
const DATABASE_KEY: &str = "database-key";

pub(crate) struct KeyringStore;

impl KeyringStore {
    fn entry(name: &str) -> Result<keyring::Entry> {
        Ok(keyring::Entry::new(SERVICE, name)?)
    }

    fn read(name: &str) -> Result<Option<SecretString>> {
        match Self::entry(name)?.get_password() {
            Ok(value) => Ok(Some(SecretString::from(value))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    fn write(name: &str, value: &str) -> Result<()> {
        Ok(Self::entry(name)?.set_password(value)?)
    }

    fn remove(name: &str) -> Result<()> {
        match Self::entry(name)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(err) => Err(err.into()),
        }
    }
}

impl CredentialStore for KeyringStore {
    fn has_credentials(&self) -> Result<bool> {
        Ok(Self::read(CLIENT_SECRET)?.is_some())
    }

    fn save(&self, credentials: &Credentials) -> Result<()> {
        Self::write(CLIENT_ID, &credentials.client_id)?;
        Self::write(CLIENT_SECRET, credentials.client_secret.expose_secret())?;
        Self::write(CUSTOMER_ID, &credentials.customer_id)
    }

    fn delete(&self) -> Result<()> {
        [CLIENT_ID, CLIENT_SECRET, CUSTOMER_ID]
            .into_iter()
            .try_for_each(Self::remove)
    }
}

impl DatabaseKeyStore for KeyringStore {
    fn load_key(&self) -> Result<Option<SecretString>> {
        Self::read(DATABASE_KEY)
    }

    fn save_key(&self, key: &SecretString) -> Result<()> {
        Self::write(DATABASE_KEY, key.expose_secret())
    }

    fn delete_key(&self) -> Result<()> {
        Self::remove(DATABASE_KEY)
    }
}
