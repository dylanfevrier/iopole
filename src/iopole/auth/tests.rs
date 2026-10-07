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
