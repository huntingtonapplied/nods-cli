//! NODS API client — a thin blocking wrapper around `reqwest` with retry.
//!
//! Auth uses the AHL standard `Authorization: ApiKey nods_...` header
//! (see nods backend/middleware/mod.rs `validate_api_key_full`).

use std::thread;
use std::time::Duration;

use anyhow::Result;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::StatusCode;
use serde_json::Value;

use crate::config;
use crate::exit_codes;

const RETRYABLE_STATUS: [u16; 5] = [429, 500, 502, 503, 504];

/// A categorized CLI error carrying the process exit code to use.
#[derive(Debug)]
pub struct CliError {
    pub code: i32,
    pub message: String,
}

impl CliError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CliError {}

pub type CliResult<T> = std::result::Result<T, CliError>;

pub struct NodsClient {
    http: Client,
    base_url: String,
    api_key: String,
    max_retries: u32,
}

impl NodsClient {
    /// Build an authenticated client. Errors (AUTH) if no API key is set.
    pub fn new(api_url_override: Option<&str>) -> CliResult<Self> {
        let api_key = config::api_key().ok_or_else(|| {
            CliError::new(
                exit_codes::AUTH,
                "Not authenticated.\n  Run: nods login\n  Or set NODS_API_KEY env var",
            )
        })?;
        Ok(Self::with_key(api_key, api_url_override))
    }

    /// Build a client with an explicit key (used by `login`/`doctor` to validate).
    pub fn with_key(api_key: String, api_url_override: Option<&str>) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(config::timeout_secs()))
            .user_agent(concat!("nods-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client");
        Self {
            http,
            base_url: config::api_url(api_url_override).trim_end_matches('/').to_string(),
            api_key,
            max_retries: config::max_retries(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    fn auth(&self, rb: RequestBuilder) -> RequestBuilder {
        rb.header("Authorization", format!("ApiKey {}", self.api_key))
            .header("Content-Type", "application/json")
    }

    fn send_with_retry(&self, build: impl Fn() -> RequestBuilder) -> CliResult<Response> {
        let mut attempt = 0;
        loop {
            let result = self.auth(build()).send();
            match result {
                Ok(resp) => {
                    let status = resp.status().as_u16();
                    if RETRYABLE_STATUS.contains(&status) && attempt < self.max_retries {
                        let delay = backoff(attempt, resp.headers().get("Retry-After"));
                        thread::sleep(delay);
                        attempt += 1;
                        continue;
                    }
                    return Ok(resp);
                }
                Err(e) => {
                    if (e.is_timeout() || e.is_connect()) && attempt < self.max_retries {
                        thread::sleep(backoff(attempt, None));
                        attempt += 1;
                        continue;
                    }
                    return Err(CliError::new(
                        exit_codes::NETWORK,
                        format!("Network error: {e}\n  Is the API reachable at {}?", self.base_url),
                    ));
                }
            }
        }
    }

    /// GET a path under the base URL and return the decoded JSON body.
    pub fn get(&self, path: &str, query: &[(&str, String)]) -> CliResult<Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.send_with_retry(|| self.http.get(&url).query(query))?;
        self.decode(resp)
    }

    pub fn post(&self, path: &str, body: Option<&Value>) -> CliResult<Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.send_with_retry(|| {
            let rb = self.http.post(&url);
            match body {
                Some(b) => rb.json(b),
                None => rb,
            }
        })?;
        self.decode(resp)
    }

    pub fn delete(&self, path: &str) -> CliResult<Value> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self.send_with_retry(|| self.http.delete(&url))?;
        self.decode(resp)
    }

    /// Map HTTP status to exit code + a friendly hint, or return the JSON body.
    fn decode(&self, resp: Response) -> CliResult<Value> {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        let json: Value = serde_json::from_str(&text).unwrap_or(Value::Null);

        if status.is_success() {
            return Ok(json);
        }

        let detail = json
            .get("message")
            .or_else(|| json.get("error"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let (code, hint) = match status {
            StatusCode::UNAUTHORIZED => (
                exit_codes::AUTH,
                "Run: nods login   (or set NODS_API_KEY)",
            ),
            StatusCode::FORBIDDEN => (exit_codes::AUTH, "Your API key lacks the required scope (write?)."),
            StatusCode::NOT_FOUND => (exit_codes::NOT_FOUND, "Resource not found. Check the ID."),
            StatusCode::UNPROCESSABLE_ENTITY | StatusCode::BAD_REQUEST => {
                (exit_codes::VALIDATION, "Request was invalid. See `<command> --help`.")
            }
            StatusCode::TOO_MANY_REQUESTS => (exit_codes::NETWORK, "Rate limited. Try again shortly."),
            s if s.is_server_error() => (exit_codes::SERVER, "Server error. Try again later."),
            _ => (exit_codes::GENERAL, ""),
        };

        let mut msg = format!("HTTP {}", status.as_u16());
        if !detail.is_empty() {
            msg.push_str(&format!(": {detail}"));
        }
        if !hint.is_empty() {
            msg.push_str(&format!("\n  {hint}"));
        }
        Err(CliError::new(code, msg))
    }
}

fn backoff(attempt: u32, retry_after: Option<&reqwest::header::HeaderValue>) -> Duration {
    let server = retry_after
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);
    let exp = (1.0_f64) * 2f64.powi(attempt as i32);
    let secs = server.max(exp).min(30.0);
    Duration::from_secs_f64(secs)
}

/// Unwrap the `{ "success": _, "data": <...> }` envelope nods uses.
pub fn data(body: &Value) -> &Value {
    body.get("data").unwrap_or(body)
}

/// Best-effort health probe (used by `doctor`); does not require auth.
pub fn probe_health(base_url: &str) -> Result<u16> {
    let url = format!("{}/health", base_url.trim_end_matches('/'));
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    Ok(client.get(url).send()?.status().as_u16())
}
