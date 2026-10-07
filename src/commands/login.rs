use std::io::Write;

use dialoguer::{Input, Password};
use secrecy::SecretString;

use crate::Result;
use crate::credentials::{CredentialStore, Credentials};

pub(crate) fn prompt_credentials() -> Result<Credentials> {
    let client_id: String = Input::new().with_prompt("Iopole client").interact_text()?;
    let client_secret = Password::new().with_prompt("Iopole secret").interact()?;
    let customer_id: String = Input::new()
        .with_prompt("Iopole customer ID")
        .interact_text()?;

    Ok(Credentials {
        client_id: client_id.trim().to_owned(),
        client_secret: SecretString::from(client_secret),
        customer_id: customer_id.trim().to_owned(),
    })
}

pub(crate) fn execute(
    store: &impl CredentialStore,
    prompt: impl FnOnce() -> Result<Credentials>,
    out: &mut impl Write,
) -> Result<()> {
    if store.has_credentials()? {
        writeln!(
            out,
            "Already logged in. Run `iopole logout` first to use other credentials."
        )?;
        return Ok(());
    }

    store.save(&prompt()?)?;
    writeln!(out, "Credentials stored in the system keyring.")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret;

    use super::*;
    use crate::credentials::testing::InMemoryStore;

    fn credentials() -> Credentials {
        Credentials {
            client_id: "client-123".to_owned(),
            client_secret: SecretString::from("s3cr3t"),
            customer_id: "customer-456".to_owned(),
        }
    }

    fn unreachable_prompt() -> Result<Credentials> {
        panic!("prompt must not be shown when already logged in")
    }

    #[test]
    fn saves_prompted_credentials() -> Result<()> {
        let store = InMemoryStore::default();
        let mut out = Vec::new();

        execute(&store, || Ok(credentials()), &mut out)?;

        assert_eq!(
            store.saved.into_inner(),
            Some((
                "client-123".to_owned(),
                "s3cr3t".to_owned(),
                "customer-456".to_owned(),
            ))
        );
        assert_eq!(
            String::from_utf8_lossy(&out),
            "Credentials stored in the system keyring.\n"
        );
        Ok(())
    }

    #[test]
    fn suggests_logout_when_already_logged_in() -> Result<()> {
        let store = InMemoryStore::logged_in();
        let mut out = Vec::new();

        execute(&store, unreachable_prompt, &mut out)?;

        assert!(String::from_utf8_lossy(&out).contains("iopole logout"));
        assert_eq!(
            store.saved.into_inner(),
            Some((
                "existing-client".to_owned(),
                "existing".to_owned(),
                "existing-customer".to_owned(),
            ))
        );
        Ok(())
    }

    #[test]
    fn secret_is_not_leaked_by_debug() {
        let credentials = credentials();

        assert!(!format!("{credentials:?}").contains(credentials.client_secret.expose_secret()));
    }
}
