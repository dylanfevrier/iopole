use std::thread;
use std::time::{Duration, Instant};

use secrecy::ExposeSecret;
use ureq::Agent;

use super::auth::{AccessToken, TokenResponse};
use super::business_entity::{
    BusinessEntity, identifier_query, parse_entities, parse_raw_entities,
};
use super::french_directory::{DirectoryPage, DirectoryQuery, EMPTY_PAGE, parse_directory_page};
use super::model::parse_page;
use super::status_history::{StatusEvent, parse_status_history};
use super::{
    BusinessEntityDirectory, FrenchDirectory, InvoiceSearch, InvoiceStatuses, MAX_PAGE_LIMIT,
    SearchQuery,
};
use crate::credentials::Credentials;
use crate::invoice::Invoice;
use crate::{Error, Result};

const DEFAULT_AUTH_URL: &str = "https://auth.iopole.com";
const DEFAULT_AUTH_ENDPOINT: &str = "realms/iopole/protocol/openid-connect/token";
const DEFAULT_API_URL: &str = "https://api.iopole.com";
const INVOICE_SEARCH_PATH: &str = "v1.1/invoice/search";
const INVOICE_EXPAND: &str = "businessData,lastStatusData";
const BUSINESS_ENTITY_PATH: &str = "v1/config/business/entity";
const FRENCH_DIRECTORY_PATH: &str = "v1/directory/french";

/// The announced quota (1000/min) is shared with production einvoicing.
const REQUESTS_PER_MINUTE: u32 = 300;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const SERVER_ERROR_ATTEMPTS: u32 = 3;
const SERVER_ERROR_BACKOFF: Duration = Duration::from_secs(1);
const RATE_LIMIT_WAITS: u32 = 5;
const RATE_LIMIT_BACKOFF: Duration = Duration::from_secs(30);
const ERROR_BODY_EXCERPT: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Endpoints {
    token_url: String,
    api_url: String,
}

impl Endpoints {
    pub(crate) fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let var = |name, default: &str| lookup(name).unwrap_or_else(|| default.to_owned());
        Self {
            token_url: join(
                &var("IOPOLE_AUTH_URL", DEFAULT_AUTH_URL),
                &var("IOPOLE_AUTH_ENDPOINT", DEFAULT_AUTH_ENDPOINT),
            ),
            api_url: var("IOPOLE_PDP_URL", DEFAULT_API_URL),
        }
    }
}

fn join(base: &str, path: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        path.trim_start_matches('/')
    )
}

pub(crate) struct HttpClient {
    agent: Agent,
    endpoints: Endpoints,
    credentials: Credentials,
    token: Option<AccessToken>,
    pacer: Pacer,
}

impl HttpClient {
    pub(crate) fn new(endpoints: Endpoints, credentials: Credentials) -> Self {
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(REQUEST_TIMEOUT))
            .build()
            .into();
        Self {
            agent,
            endpoints,
            credentials,
            token: None,
            pacer: Pacer::per_minute(REQUESTS_PER_MINUTE),
        }
    }

    fn authorization(&mut self) -> Result<String> {
        if let Some(token) = &self.token
            && token.is_usable_at(Instant::now())
        {
            return Ok(token.authorization_header());
        }
        let token = self.request_token()?;
        let authorization = token.authorization_header();
        self.token = Some(token);
        Ok(authorization)
    }

    fn request_token(&self) -> Result<AccessToken> {
        let issued_at = Instant::now();
        let mut response = self.agent.post(&self.endpoints.token_url).send_form([
            ("grant_type", "client_credentials"),
            ("client_id", &self.credentials.client_id),
            (
                "client_secret",
                self.credentials.client_secret.expose_secret(),
            ),
        ])?;
        if !response.status().is_success() {
            return Err(Error::Authentication(response.status().as_u16()));
        }
        let token: TokenResponse = serde_json::from_str(&response.body_mut().read_to_string()?)?;
        Ok(token.into_token(issued_at))
    }

    fn get(&mut self, path: &str, query: &[(&str, String)]) -> Result<String> {
        let url = join(&self.endpoints.api_url, path);
        let mut attempts = Attempts::default();
        loop {
            let authorization = self.authorization()?;
            self.pacer.wait();
            let next = match self.send_get(&url, &authorization, query) {
                Ok(reply) if (200..300).contains(&reply.status) => return Ok(reply.body),
                Ok(reply) => attempts.after_reply(&reply)?,
                Err(err) if is_transient(&err) => attempts.after_transient_failure(err)?,
                Err(err) => return Err(err.into()),
            };
            match next {
                Next::Reauthenticate => self.token = None,
                Next::Wait(delay) => thread::sleep(delay),
            }
        }
    }

    fn send_get(
        &self,
        url: &str,
        authorization: &str,
        query: &[(&str, String)],
    ) -> std::result::Result<Reply, ureq::Error> {
        let mut response = self
            .agent
            .get(url)
            .header("Authorization", authorization)
            .header("customer-id", &self.credentials.customer_id)
            .header("Accept", "application/json")
            .query_pairs(query.iter().map(|(name, value)| (*name, value.as_str())))
            .call()?;
        Ok(Reply {
            status: response.status().as_u16(),
            retry_after: response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.trim().parse().ok())
                .map(Duration::from_secs),
            body: response.body_mut().read_to_string()?,
        })
    }
}

impl InvoiceSearch for HttpClient {
    fn search(&mut self, query: &SearchQuery, offset: u32, limit: u32) -> Result<Vec<Invoice>> {
        let body = self.get(
            INVOICE_SEARCH_PATH,
            &[
                ("q", query.to_string()),
                ("expand", INVOICE_EXPAND.to_owned()),
                ("offset", offset.to_string()),
                ("limit", limit.to_string()),
            ],
        )?;
        parse_page(&body)
    }
}

impl BusinessEntityDirectory for HttpClient {
    fn find(&mut self, identifier_value: &str) -> Result<Vec<BusinessEntity>> {
        self.entity_page(identifier_value)?
            .map_or_else(|| Ok(Vec::new()), |body| parse_entities(&body))
    }

    fn find_raw(&mut self, identifier_value: &str) -> Result<Vec<serde_json::Value>> {
        self.entity_page(identifier_value)?
            .map_or_else(|| Ok(Vec::new()), |body| parse_raw_entities(&body))
    }
}

impl HttpClient {
    /// `None` when no entity matches.
    fn entity_page(&mut self, identifier_value: &str) -> Result<Option<String>> {
        let reply = self.get(
            BUSINESS_ENTITY_PATH,
            &[
                ("q", identifier_query(identifier_value)),
                ("limit", MAX_PAGE_LIMIT.to_string()),
            ],
        );
        match reply {
            Ok(body) => Ok(Some(body)),
            // Documented answer when no entity matches the query.
            Err(Error::UnexpectedStatus { status: 404, .. }) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

impl InvoiceStatuses for HttpClient {
    fn status_history(&mut self, invoice_id: &str) -> Result<Option<Vec<StatusEvent>>> {
        self.status_history_body(invoice_id)?
            .map(|body| parse_status_history(&body))
            .transpose()
    }

    fn raw_status_history(&mut self, invoice_id: &str) -> Result<Option<serde_json::Value>> {
        self.status_history_body(invoice_id)?
            .map(|body| Ok(serde_json::from_str(&body)?))
            .transpose()
    }
}

impl HttpClient {
    fn status_history_body(&mut self, invoice_id: &str) -> Result<Option<String>> {
        match self.get(&format!("v1/invoice/{invoice_id}/status-history"), &[]) {
            Ok(body) => Ok(Some(body)),
            Err(Error::UnexpectedStatus { status: 404, .. }) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

impl FrenchDirectory for HttpClient {
    fn search_french(
        &mut self,
        query: &DirectoryQuery,
        offset: u32,
        limit: u32,
    ) -> Result<DirectoryPage> {
        let reply = self.get(
            FRENCH_DIRECTORY_PATH,
            &[
                ("q", query.to_string()),
                ("withPlatformDetails", "true".to_owned()),
                ("offset", offset.to_string()),
                ("limit", limit.to_string()),
            ],
        );
        match reply {
            Ok(body) => parse_directory_page(body),
            Err(Error::UnexpectedStatus { status: 404, .. }) => {
                parse_directory_page(EMPTY_PAGE.to_owned())
            }
            Err(err) => Err(err),
        }
    }
}

struct Reply {
    status: u16,
    retry_after: Option<Duration>,
    body: String,
}

#[derive(Debug, PartialEq, Eq)]
enum Next {
    Reauthenticate,
    Wait(Duration),
}

#[derive(Default)]
struct Attempts {
    reauthenticated: bool,
    server_errors: u32,
    rate_limited: u32,
}

impl Attempts {
    fn after_reply(&mut self, reply: &Reply) -> Result<Next> {
        match reply.status {
            400 => Err(Error::InvalidQuery(excerpt(&reply.body))),
            401 | 403 if !self.reauthenticated => {
                self.reauthenticated = true;
                Ok(Next::Reauthenticate)
            }
            401 | 403 => Err(Error::Unauthorized(reply.status)),
            429 => {
                self.rate_limited += 1;
                if self.rate_limited > RATE_LIMIT_WAITS {
                    return Err(Error::RateLimited);
                }
                Ok(Next::Wait(reply.retry_after.unwrap_or_else(|| {
                    exponential(RATE_LIMIT_BACKOFF, self.rate_limited)
                })))
            }
            500..=599 => self.after_server_error(Error::Server(reply.status)),
            status => Err(Error::UnexpectedStatus {
                status,
                code: error_code(&reply.body),
            }),
        }
    }

    fn after_transient_failure(&mut self, err: ureq::Error) -> Result<Next> {
        self.after_server_error(err.into())
    }

    fn after_server_error(&mut self, err: Error) -> Result<Next> {
        self.server_errors += 1;
        if self.server_errors >= SERVER_ERROR_ATTEMPTS {
            return Err(err);
        }
        Ok(Next::Wait(exponential(
            SERVER_ERROR_BACKOFF,
            self.server_errors,
        )))
    }
}

fn exponential(base: Duration, attempt: u32) -> Duration {
    base.saturating_mul(1 << attempt.saturating_sub(1).min(16))
}

fn is_transient(err: &ureq::Error) -> bool {
    matches!(
        err,
        ureq::Error::Io(_) | ureq::Error::Timeout(_) | ureq::Error::ConnectionFailed
    )
}

/// Iopole reports business refusals (even with a 404) with a `code` field.
fn error_code(body: &str) -> Option<String> {
    let body: serde_json::Value = serde_json::from_str(body).ok()?;
    body.get("code")?.as_str().map(str::to_owned)
}

fn excerpt(body: &str) -> String {
    body.chars().take(ERROR_BODY_EXCERPT).collect()
}

struct Pacer {
    interval: Duration,
    next: Option<Instant>,
}

impl Pacer {
    fn per_minute(requests: u32) -> Self {
        Self {
            interval: Duration::from_secs(60) / requests.max(1),
            next: None,
        }
    }

    fn wait(&mut self) {
        let now = Instant::now();
        if let Some(next) = self.next
            && next > now
        {
            thread::sleep(next - now);
        }
        self.next = Some(Instant::now() + self.interval);
    }
}

#[cfg(test)]
mod tests {
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
}
