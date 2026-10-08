use std::cell::RefCell;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use etcetera::BaseStrategy;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::credentials::{CredentialStore, Credentials};
use crate::database::{self, DatabaseKeyStore};
use crate::{Error, Result};

const APP_DIR: &str = "iopole";
const SECRETS_FILE: &str = "secrets.json";

/// All secrets share a single file, in clear but readable only by its owner.
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

pub(crate) struct SecretsFile {
    path: PathBuf,
    cache: RefCell<Option<Secrets>>,
}

impl SecretsFile {
    pub(crate) fn at(path: PathBuf) -> Self {
        Self {
            path,
            cache: RefCell::new(None),
        }
    }

    pub(crate) fn default_path() -> Result<PathBuf> {
        let config_dir = etcetera::choose_base_strategy()?.config_dir();
        Ok(config_dir.join(APP_DIR).join(SECRETS_FILE))
    }

    fn secrets(&self) -> Result<Secrets> {
        if let Some(secrets) = self.cache.borrow().as_ref() {
            return Ok(secrets.clone());
        }
        let secrets = match std::fs::read_to_string(&self.path) {
            Ok(json) => {
                let json = SecretString::from(json);
                refuse_if_exposed(&self.path)?;
                serde_json::from_str(json.expose_secret())?
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Secrets::default(),
            Err(err) => return Err(err.into()),
        };
        *self.cache.borrow_mut() = Some(secrets.clone());
        Ok(secrets)
    }

    fn update(&self, change: impl FnOnce(&mut Secrets)) -> Result<()> {
        let mut secrets = self.secrets()?;
        change(&mut secrets);
        if secrets.is_empty() {
            match std::fs::remove_file(&self.path) {
                Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err.into()),
                _ => {}
            }
        } else {
            self.write(&secrets)?;
        }
        *self.cache.borrow_mut() = Some(secrets);
        Ok(())
    }

    /// Written aside then renamed, so that an interrupted write never loses
    /// the database key.
    fn write(&self, secrets: &Secrets) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            database::create_private_dir(dir)?;
        }
        let mut pending = self.path.as_os_str().to_owned();
        pending.push(".tmp");
        let pending = PathBuf::from(pending);
        let _ = std::fs::remove_file(&pending);

        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let json = SecretString::from(serde_json::to_string_pretty(secrets)?);
        let mut file = options.open(&pending)?;
        file.write_all(json.expose_secret().as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&pending, &self.path)?;
        Ok(())
    }
}

/// Clear secrets are only acceptable when nobody else can read them.
fn refuse_if_exposed(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if std::fs::metadata(path)?.permissions().mode() & 0o077 != 0 {
            return Err(Error::ExposedSecrets(path.to_owned()));
        }
    }
    Ok(())
}

impl CredentialStore for SecretsFile {
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

    fn location(&self) -> String {
        self.path.display().to_string()
    }
}

impl DatabaseKeyStore for SecretsFile {
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

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        tmp: tempfile::TempDir,
    }

    impl Fixture {
        fn new() -> Result<Self> {
            Ok(Self {
                tmp: tempfile::tempdir()?,
            })
        }

        fn path(&self) -> PathBuf {
            self.tmp.path().join("iopole").join(SECRETS_FILE)
        }

        /// A fresh store, as the next run of the command reads it.
        fn store(&self) -> SecretsFile {
            SecretsFile::at(self.path())
        }
    }

    fn credentials() -> Credentials {
        Credentials {
            client_id: "client-123".to_owned(),
            client_secret: SecretString::from("s3cr3t"),
            customer_id: "customer-456".to_owned(),
        }
    }

    #[test]
    fn keeps_credentials_and_database_key_across_runs() -> Result<()> {
        let fixture = Fixture::new()?;
        fixture.store().save(&credentials())?;
        fixture.store().save_key(&SecretString::from("abcd"))?;

        let store = fixture.store();
        let loaded = store.load()?.unwrap_or_else(|| panic!("credentials lost"));

        assert_eq!(loaded.client_id, "client-123");
        assert_eq!(loaded.client_secret.expose_secret(), "s3cr3t");
        assert_eq!(loaded.customer_id, "customer-456");
        assert_eq!(
            store.load_key()?.map(|key| key.expose_secret().to_owned()),
            Some("abcd".to_owned())
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn only_the_owner_can_read_the_file() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let fixture = Fixture::new()?;

        fixture.store().save(&credentials())?;

        let mode = |path: &Path| -> Result<u32> {
            Ok(std::fs::metadata(path)?.permissions().mode() & 0o777)
        };
        assert_eq!(mode(&fixture.path())?, 0o600);
        assert_eq!(
            mode(fixture.path().parent().unwrap_or(Path::new("/")))?,
            0o700
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_file_readable_by_others() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let fixture = Fixture::new()?;
        fixture.store().save(&credentials())?;
        std::fs::set_permissions(fixture.path(), std::fs::Permissions::from_mode(0o644))?;

        let result = fixture.store().load();

        assert!(matches!(result, Err(Error::ExposedSecrets(_))));
        Ok(())
    }

    #[test]
    fn logout_keeps_the_database_key_until_it_is_deleted() -> Result<()> {
        let fixture = Fixture::new()?;
        fixture.store().save(&credentials())?;
        fixture.store().save_key(&SecretString::from("abcd"))?;

        fixture.store().delete()?;

        assert!(!fixture.store().has_credentials()?);
        assert!(fixture.store().load_key()?.is_some());
        fixture.store().delete_key()?;
        assert!(!fixture.path().exists());
        Ok(())
    }

    #[test]
    fn nothing_stored_without_file() -> Result<()> {
        let fixture = Fixture::new()?;

        assert!(!fixture.store().has_credentials()?);
        assert!(fixture.store().load_key()?.is_none());
        assert!(!fixture.path().exists());
        Ok(())
    }
}
