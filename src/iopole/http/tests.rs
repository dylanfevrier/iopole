use super::*;

fn reply(status: u16) -> Reply {
    Reply {
        status,
        retry_after: None,
        body: String::new(),
    }
}

#[test]
fn defaults_to_production_urls() {
    let endpoints = Endpoints::from_lookup(|_| None);

    assert_eq!(
        endpoints.token_url,
        "https://auth.iopole.com/realms/iopole/protocol/openid-connect/token"
    );
    assert_eq!(
        join(&endpoints.api_url, INVOICE_SEARCH_PATH),
        "https://api.iopole.com/v1.1/invoice/search"
    );
}

#[test]
fn urls_can_target_preproduction() {
    let endpoints = Endpoints::from_lookup(|name| match name {
        "IOPOLE_AUTH_URL" => Some("https://auth.ppd.iopole.fr/".to_owned()),
        "IOPOLE_PDP_URL" => Some("https://api.ppd.iopole.fr".to_owned()),
        _ => None,
    });

    assert_eq!(
        endpoints.token_url,
        "https://auth.ppd.iopole.fr/realms/iopole/protocol/openid-connect/token"
    );
    assert_eq!(
        join(&endpoints.api_url, BUSINESS_ENTITY_PATH),
        "https://api.ppd.iopole.fr/v1/config/business/entity"
    );
}

#[test]
fn invalid_query_is_not_retried() {
    let mut attempts = Attempts::default();
    let reply = Reply {
        body: "Unable to parse correctly query search".to_owned(),
        ..reply(400)
    };

    assert!(
        matches!(attempts.after_reply(&reply), Err(Error::InvalidQuery(body)) if body.contains("parse"))
    );
}

#[test]
fn reauthenticates_once_then_fails() -> Result<()> {
    let mut attempts = Attempts::default();

    assert_eq!(attempts.after_reply(&reply(401))?, Next::Reauthenticate);
    assert!(matches!(
        attempts.after_reply(&reply(403)),
        Err(Error::Unauthorized(403))
    ));
    Ok(())
}

#[test]
fn server_errors_are_tried_three_times() -> Result<()> {
    let mut attempts = Attempts::default();

    assert_eq!(
        attempts.after_reply(&reply(502))?,
        Next::Wait(Duration::from_secs(1))
    );
    assert_eq!(
        attempts.after_reply(&reply(503))?,
        Next::Wait(Duration::from_secs(2))
    );
    assert!(matches!(
        attempts.after_reply(&reply(500)),
        Err(Error::Server(500))
    ));
    Ok(())
}

#[test]
fn rate_limit_honours_retry_after() -> Result<()> {
    let mut attempts = Attempts::default();
    let reply = Reply {
        retry_after: Some(Duration::from_secs(7)),
        ..reply(429)
    };

    assert_eq!(
        attempts.after_reply(&reply)?,
        Next::Wait(Duration::from_secs(7))
    );
    Ok(())
}

#[test]
fn rate_limit_backs_off_exponentially_then_gives_up() -> Result<()> {
    let mut attempts = Attempts::default();

    for expected in [30, 60, 120, 240, 480] {
        assert_eq!(
            attempts.after_reply(&reply(429))?,
            Next::Wait(Duration::from_secs(expected))
        );
    }
    assert!(matches!(
        attempts.after_reply(&reply(429)),
        Err(Error::RateLimited)
    ));
    Ok(())
}

#[test]
fn business_refusal_reports_its_code() {
    let mut attempts = Attempts::default();
    let reply = Reply {
        body: r#"{"code":"CUSTOMER_NOT_FOUND","message":"..."}"#.to_owned(),
        ..reply(404)
    };

    assert!(matches!(
        attempts.after_reply(&reply),
        Err(Error::UnexpectedStatus { status: 404, code: Some(code) }) if code == "CUSTOMER_NOT_FOUND"
    ));
}

#[test]
fn pacer_spaces_requests() {
    let pacer = Pacer::per_minute(300);

    assert_eq!(pacer.interval, Duration::from_millis(200));
}
