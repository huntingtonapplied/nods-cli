//! Command handlers. Each returns `CliResult<()>`; the categorized `CliError`
//! carries the process exit code.

use std::io::{self, Write};

use serde_json::{json, Value};

use crate::client::{self, CliError, CliResult, NodsClient};
use crate::config;
use crate::exit_codes;
use crate::output::{self, Format};

fn ok() -> CliResult<()> {
    Ok(())
}

/// Print a JSON value either as JSON or via a fallback renderer.
fn emit(fmt: Format, value: &Value, render: impl FnOnce(&Value)) {
    if fmt.is_json() {
        output::print_json(value);
    } else {
        render(value);
    }
}

// --------------------------------------------------------------------------
// login
// --------------------------------------------------------------------------

pub fn login(api_url: Option<&str>, api_key: Option<String>, fmt: Format) -> CliResult<()> {
    let key = match api_key {
        Some(k) => k.trim().to_string(),
        None => prompt_secret("Enter your NODS API key (starts with nods_): ")?,
    };
    if key.is_empty() {
        return Err(CliError::new(exit_codes::VALIDATION, "No API key provided."));
    }

    // Validate the key against the account profile before persisting.
    let validated = {
        let probe = NodsClient::with_key(key.clone(), api_url);
        probe.get("/v1/account/profile", &[])
    };
    match validated {
        Ok(_) => {}
        Err(e) if e.code == exit_codes::AUTH => {
            return Err(CliError::new(exit_codes::AUTH, "API key rejected by server (401)."));
        }
        // A network/server hiccup shouldn't block storing a key the user pasted.
        Err(e) => eprintln!("Warning: could not verify key ({e}); storing anyway."),
    }

    config::set_api_key(&key)
        .map_err(|e| CliError::new(exit_codes::GENERAL, format!("Failed to save config: {e}")))?;

    if fmt.is_json() {
        output::print_json(&json!({"ok": true, "config_file": config::config_file().display().to_string()}));
    } else {
        println!("Saved API key to {}", config::config_file().display());
    }
    ok()
}

fn prompt_secret(prompt: &str) -> CliResult<String> {
    print!("{prompt}");
    io::stdout().flush().ok();
    let mut line = String::new();
    io::stdin()
        .read_line(&mut line)
        .map_err(|e| CliError::new(exit_codes::GENERAL, format!("read error: {e}")))?;
    Ok(line.trim().to_string())
}

// --------------------------------------------------------------------------
// status
// --------------------------------------------------------------------------

pub fn status(api_url: Option<&str>, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let profile = client::data(&c.get("/v1/account/profile", &[])?).clone();
    let sessions = c
        .get("/v1/sessions", &[("status", "active".into()), ("limit", "100".into())])
        .map(|b| client::data(&b).clone())
        .unwrap_or(Value::Array(vec![]));
    let active = sessions.as_array().map(|a| a.len()).unwrap_or(0);

    if fmt.is_json() {
        output::print_json(&json!({"profile": profile, "active_sessions": active}));
        return ok();
    }

    let email = profile.get("email").and_then(|v| v.as_str()).unwrap_or("-");
    let name = profile
        .get("name")
        .or_else(|| profile.get("username"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    output::print_kv(&[
        ("API URL", c.base_url().to_string()),
        ("User", name.to_string()),
        ("Email", email.to_string()),
        ("Active sessions", active.to_string()),
    ]);
    ok()
}

// --------------------------------------------------------------------------
// doctor
// --------------------------------------------------------------------------

pub fn doctor(api_url: Option<&str>, fmt: Format) -> CliResult<()> {
    let mut checks: Vec<Value> = Vec::new();
    let mut add = |label: &str, pass: bool, detail: String| {
        checks.push(json!({"check": label, "ok": pass, "detail": detail}));
        pass
    };

    add("CLI version", true, env!("CARGO_PKG_VERSION").to_string());

    let cfg_file = config::config_file();
    add("Config file exists", cfg_file.exists(), cfg_file.display().to_string());

    let cfg = config::load();
    let warnings = config::validate(&cfg);
    add(
        "Config is valid",
        warnings.is_empty(),
        if warnings.is_empty() { "ok".into() } else { warnings.join("; ") },
    );

    let key = config::api_key();
    let has_key = key.is_some();
    let key_src = if config::api_key_from_env() {
        "env var"
    } else if has_key {
        "config file"
    } else {
        "not set"
    };
    add("API key configured", has_key, key_src.to_string());

    let base = config::api_url(api_url);
    let health = client::probe_health(&base);
    let reachable = matches!(health, Ok(code) if code < 500);
    add(
        "API reachable",
        reachable,
        match &health {
            Ok(code) => format!("{base} (HTTP {code})"),
            Err(e) => format!("{base} ({e})"),
        },
    );

    let key_valid = if has_key && reachable {
        NodsClient::new(api_url)
            .and_then(|c| c.get("/v1/account/profile", &[]))
            .is_ok()
    } else {
        false
    };
    add(
        "API key valid",
        key_valid,
        if !has_key {
            "no key".into()
        } else if key_valid {
            "verified".into()
        } else {
            "invalid or unreachable".into()
        },
    );

    let passed = checks.iter().filter(|c| c["ok"].as_bool().unwrap_or(false)).count();
    let total = checks.len();

    if fmt.is_json() {
        output::print_json(&json!({
            "version": env!("CARGO_PKG_VERSION"),
            "checks": checks,
            "passed": passed,
            "total": total,
        }));
    } else {
        println!("NODS CLI v{}\n", env!("CARGO_PKG_VERSION"));
        for c in &checks {
            let icon = if c["ok"].as_bool().unwrap_or(false) { "ok " } else { "FAIL" };
            let label = c["check"].as_str().unwrap_or("");
            let detail = c["detail"].as_str().unwrap_or("");
            println!("  [{icon}] {label}  ({detail})");
        }
        println!("\n{passed}/{total} checks passed.");
    }

    if passed != total {
        return Err(CliError::new(exit_codes::GENERAL, format!("{}/{} checks failed", total - passed, total)));
    }
    ok()
}

// --------------------------------------------------------------------------
// metrics
// --------------------------------------------------------------------------

pub fn metrics(api_url: Option<&str>, path: &str, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let body = c.get(path, &[])?;
    let data = client::data(&body).clone();

    if fmt.is_json() {
        output::print_json(&data);
        return ok();
    }

    match &data {
        Value::Array(rows) => {
            // Render array-shaped analytics (backends / daily / errors / top-configs).
            if let Some(first) = rows.first() {
                let cols: Vec<(String, String)> = first
                    .as_object()
                    .map(|o| o.keys().map(|k| (titlecase(k), k.clone())).collect())
                    .unwrap_or_default();
                let col_refs: Vec<(&str, &str)> =
                    cols.iter().map(|(h, k)| (h.as_str(), k.as_str())).collect();
                output::print_table(rows, &col_refs);
            } else {
                println!("(no data)");
            }
        }
        Value::Object(obj) => {
            let pairs: Vec<(&str, String)> = obj
                .iter()
                .map(|(k, v)| (k.as_str(), scalar(v)))
                .collect();
            output::print_kv(&pairs);
        }
        other => println!("{other}"),
    }
    ok()
}

// --------------------------------------------------------------------------
// configs
// --------------------------------------------------------------------------

pub fn configs_list(api_url: Option<&str>, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let body = c.get("/v1/configs", &[])?;
    let data = client::data(&body).clone();
    emit(fmt, &data, |v| {
        let rows = v.as_array().cloned().unwrap_or_default();
        output::print_table(
            &rows,
            &[
                ("ID", "id"),
                ("NAME", "name"),
                ("BACKEND", "backend"),
                ("USES", "use_count"),
                ("SYNC", "sync_status"),
            ],
        );
    });
    ok()
}

pub fn configs_show(api_url: Option<&str>, id: &str, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let body = c.get(&format!("/v1/configs/{id}"), &[])?;
    print_record(fmt, client::data(&body));
    ok()
}

pub fn configs_validate(api_url: Option<&str>, id: &str, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let body = c.post(&format!("/v1/configs/{id}/validate"), None)?;
    let data = client::data(&body).clone();
    let valid = data.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
    if fmt.is_json() {
        output::print_json(&data);
    } else {
        println!("valid: {valid}");
        for field in ["errors", "warnings"] {
            if let Some(arr) = data.get(field).and_then(|v| v.as_array()) {
                if !arr.is_empty() {
                    println!("{field}:");
                    for item in arr {
                        println!("  - {}", scalar(item));
                    }
                }
            }
        }
    }
    if !valid {
        return Err(CliError::new(exit_codes::VALIDATION, "config is invalid"));
    }
    ok()
}

pub fn configs_delete(api_url: Option<&str>, id: &str, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let body = c.delete(&format!("/v1/configs/{id}"))?;
    report_mutation(fmt, &body, &format!("Deleted config {id}"));
    ok()
}

// --------------------------------------------------------------------------
// sessions
// --------------------------------------------------------------------------

pub fn sessions_list(
    api_url: Option<&str>,
    status: Option<&str>,
    limit: u32,
    fmt: Format,
) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let mut q: Vec<(&str, String)> = vec![("limit", limit.to_string())];
    if let Some(s) = status {
        q.push(("status", s.to_string()));
    }
    let body = c.get("/v1/sessions", &q)?;
    let data = client::data(&body).clone();
    emit(fmt, &data, |v| {
        let rows = v.as_array().cloned().unwrap_or_default();
        output::print_table(
            &rows,
            &[
                ("ID", "id"),
                ("CONFIG", "config_name"),
                ("BACKEND", "backend"),
                ("STATUS", "status"),
                ("STARTED", "started_at"),
            ],
        );
    });
    ok()
}

pub fn sessions_show(api_url: Option<&str>, id: &str, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let body = c.get(&format!("/v1/sessions/{id}"), &[])?;
    print_record(fmt, client::data(&body));
    ok()
}

pub fn sessions_launch(api_url: Option<&str>, config_id: &str, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let body = c.post(&format!("/v1/sessions/launch/{config_id}"), None)?;
    let data = client::data(&body).clone();
    if fmt.is_json() {
        output::print_json(&data);
    } else {
        let sid = data
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        println!("Launched session {sid} from config {config_id}");
    }
    ok()
}

pub fn sessions_kill(api_url: Option<&str>, id: &str, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let body = c.delete(&format!("/v1/sessions/{id}"))?;
    report_mutation(fmt, &body, &format!("Stopped session {id}"));
    ok()
}

// --------------------------------------------------------------------------
// templates / events / resources
// --------------------------------------------------------------------------

pub fn templates_list(api_url: Option<&str>, fmt: Format) -> CliResult<()> {
    // Public library: no login needed (GET /v1/templates is unauthenticated).
    let c = NodsClient::public(api_url);
    let body = c.get("/v1/templates", &[])?;
    let data = client::data(&body).clone();
    emit(fmt, &data, |v| {
        let rows = v.as_array().cloned().unwrap_or_default();
        output::print_table(
            &rows,
            &[("ID", "id"), ("NAME", "title"), ("BACKEND", "backend")],
        );
    });
    ok()
}

pub fn templates_show(api_url: Option<&str>, id: &str, fmt: Format) -> CliResult<()> {
    let c = NodsClient::public(api_url);
    let body = c.get(&format!("/v1/templates/{id}"), &[])?;
    let d = client::data(&body);
    print_record(fmt, d.get("template").unwrap_or(d));
    ok()
}

pub fn events(api_url: Option<&str>, etype: Option<&str>, limit: u32, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let mut q: Vec<(&str, String)> = vec![("limit", limit.to_string())];
    if let Some(t) = etype {
        q.push(("type", t.to_string()));
    }
    let body = c.get("/v1/events", &q)?;
    let data = client::data(&body).clone();
    emit(fmt, &data, |v| {
        let rows = v.as_array().cloned().unwrap_or_default();
        output::print_table(
            &rows,
            &[
                ("TYPE", "type"),
                ("MESSAGE", "message"),
                ("CREATED", "created_at"),
            ],
        );
    });
    ok()
}

pub fn resources(api_url: Option<&str>, fmt: Format) -> CliResult<()> {
    let c = NodsClient::new(api_url)?;
    let body = c.get("/v1/sessions/system/resources", &[])?;
    print_record(fmt, client::data(&body));
    ok()
}

// --------------------------------------------------------------------------
// helpers
// --------------------------------------------------------------------------

fn print_record(fmt: Format, data: &Value) {
    if fmt.is_json() {
        output::print_json(data);
        return;
    }
    match data.as_object() {
        Some(obj) => {
            let pairs: Vec<(&str, String)> =
                obj.iter().map(|(k, v)| (k.as_str(), scalar(v))).collect();
            output::print_kv(&pairs);
        }
        None => output::print_json(data),
    }
}

fn report_mutation(fmt: Format, body: &Value, human: &str) {
    if fmt.is_json() {
        output::print_json(body);
    } else {
        let msg = body.get("message").and_then(|v| v.as_str()).unwrap_or(human);
        println!("{msg}");
    }
}

fn scalar(v: &Value) -> String {
    match v {
        Value::Null => "-".to_string(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn titlecase(s: &str) -> String {
    s.replace('_', " ").to_uppercase()
}
