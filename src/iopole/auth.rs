use std::time::{Duration, Instant};

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

const RENEWAL_MARGIN: Duration = Duration::from_secs(60);

/// There is no refresh token: a new one is requested with the same credentials.
pub(super) struct AccessToken {
    value: SecretString,
    renew_at: Instant,
}

impl AccessToken {
    pub(super) fn is_usable_at(&self, now: Instant) -> bool {
        now < self.renew_at
    }

    pub(super) fn authorization_header(&self) -> String {
        format!("Bearer {}", self.value.expose_secret())
    }
}

#[derive(Deserialize)]
pub(super) struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

impl TokenResponse {
    pub(super) fn into_token(self, issued_at: Instant) -> AccessToken {
        let lifetime = Duration::from_secs(self.expires_in);
        AccessToken {
            value: SecretString::from(self.access_token),
            renew_at: issued_at + lifetime.saturating_sub(RENEWAL_MARGIN.min(lifetime / 2)),
        }
    }
}

#[cfg(test)]
mod tests;
