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
    fn load(&self) -> Result<Option<Credentials>>;
    fn save(&self, credentials: &Credentials) -> Result<()>;
    fn delete(&self) -> Result<()>;
}

#[cfg(test)]
pub(crate) mod testing;
