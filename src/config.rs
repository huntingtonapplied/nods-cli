//! Configuration management — reads/writes `~/.nods/config.yaml`.
//!
//! Precedence for every value is: environment variable > config file > default.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub const DEFAULT_API_URL: &str = "https://api.nods.dev";
pub const ENV_API_KEY: &str = "NODS_API_KEY";
pub const ENV_API_URL: &str = "NODS_API_URL";
pub const ENV_TIMEOUT: &str = "NODS_TIMEOUT";
pub const ENV_MAX_RETRIES: &str = "NODS_MAX_RETRIES";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,

    /// Any unrecognized keys, captured so we can warn the user.
    #[serde(flatten)]
    pub extra: serde_yaml::Mapping,
}

pub fn config_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".nods")
}

pub fn config_file() -> PathBuf {
    config_dir().join("config.yaml")
}

pub fn load() -> Config {
    let path = config_file();
    if !path.exists() {
        return Config::default();
    }
    match fs::read_to_string(&path) {
        Ok(raw) => serde_yaml::from_str(&raw).unwrap_or_default(),
        Err(_) => Config::default(),
    }
}

pub fn save(cfg: &Config) -> Result<()> {
    let dir = config_dir();
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let raw = serde_yaml::to_string(cfg).context("serializing config")?;
    let path = config_file();
    fs::write(&path, raw).with_context(|| format!("writing {}", path.display()))?;
    // Best-effort: tighten perms on the file holding the API key.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// Resolve the API key: `NODS_API_KEY` env wins, else config file.
pub fn api_key() -> Option<String> {
    if let Ok(k) = std::env::var(ENV_API_KEY) {
        if !k.is_empty() {
            return Some(k);
        }
    }
    load().api_key
}

/// True when the API key came from the environment (for `doctor` reporting).
pub fn api_key_from_env() -> bool {
    std::env::var(ENV_API_KEY).map(|v| !v.is_empty()).unwrap_or(false)
}

/// Resolve the API base URL: env > config > default.
pub fn api_url(cli_override: Option<&str>) -> String {
    if let Some(u) = cli_override {
        return u.to_string();
    }
    if let Ok(u) = std::env::var(ENV_API_URL) {
        if !u.is_empty() {
            return u;
        }
    }
    load().api_url.unwrap_or_else(|| DEFAULT_API_URL.to_string())
}

pub fn timeout_secs() -> u64 {
    if let Ok(v) = std::env::var(ENV_TIMEOUT) {
        if let Ok(n) = v.parse() {
            return n;
        }
    }
    load().timeout.unwrap_or(60)
}

pub fn max_retries() -> u32 {
    std::env::var(ENV_MAX_RETRIES)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(3)
}

pub fn set_api_key(key: &str) -> Result<()> {
    let mut cfg = load();
    cfg.api_key = Some(key.to_string());
    save(&cfg)
}

/// Return human-readable warnings about the on-disk config (typos, bad URL).
pub fn validate(cfg: &Config) -> Vec<String> {
    let mut warnings = Vec::new();
    for (k, _) in &cfg.extra {
        if let Some(key) = k.as_str() {
            warnings.push(format!("Unknown config key: '{key}'"));
        }
    }
    if let Some(url) = &cfg.api_url {
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            warnings.push("api_url should start with http:// or https://".to_string());
        }
    }
    warnings
}
