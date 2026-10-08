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
mod tests {
    use super::*;

    fn token(expires_in: u64, issued_at: Instant) -> AccessToken {
        TokenResponse {
            access_token: "eyJ".to_owned(),
            expires_in,
        }
        .into_token(issued_at)
    }

    #[test]
    fn fresh_token_is_usable() {
        let now = Instant::now();

        assert!(token(300, now).is_usable_at(now));
    }

    #[test]
    fn token_is_renewed_one_minute_before_expiry() {
        let now = Instant::now();
        let token = token(300, now);

        assert!(token.is_usable_at(now + Duration::from_secs(239)));
        assert!(!token.is_usable_at(now + Duration::from_secs(240)));
    }

    #[test]
    fn short_lived_token_is_used_for_half_its_lifetime() {
        let now = Instant::now();
        let token = token(30, now);

        assert!(token.is_usable_at(now + Duration::from_secs(14)));
        assert!(!token.is_usable_at(now + Duration::from_secs(15)));
    }
}
