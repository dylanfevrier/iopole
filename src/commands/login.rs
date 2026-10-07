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
mod tests;
