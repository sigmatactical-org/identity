//! Environment configuration. Prefer `IDENTITY_*` variables; `RIDSER_*` is accepted as a
//! deprecated fallback for upstream compatibility.

use std::env;

use anyhow::{Context, Result, bail};

const MIN_SESSION_SECRET_BYTES: usize = 32;

/// Read `IDENTITY_{name}` or fall back to deprecated `RIDSER_{name}`.
pub fn var(name: &str) -> Result<String> {
    let identity = format!("IDENTITY_{name}");
    if let Ok(v) = env::var(&identity) {
        return Ok(v);
    }
    let legacy = format!("RIDSER_{name}");
    env::var(&legacy).with_context(|| format!("missing {identity} (or deprecated {legacy})"))
}

pub fn var_optional(name: &str) -> Option<String> {
    let identity = format!("IDENTITY_{name}");
    env::var(&identity)
        .ok()
        .or_else(|| env::var(format!("RIDSER_{name}")).ok())
}

pub fn is_production() -> bool {
    matches!(
        var_optional("ENV")
            .or_else(|| env::var("RUST_ENV").ok())
            .as_deref(),
        Some("production" | "prod")
    )
}

pub fn danger_accept_invalid_certs() -> Result<bool> {
    let enabled =
        var_optional("DANGER_ACCEPT_INVALID_CERTS").is_some_and(|v| v.eq_ignore_ascii_case("true"));
    if enabled && is_production() {
        bail!(
            "IDENTITY_DANGER_ACCEPT_INVALID_CERTS must not be enabled when IDENTITY_ENV=production"
        );
    }
    Ok(enabled)
}

pub fn validate_session_secret(secret: &str) -> Result<()> {
    if secret.len() < MIN_SESSION_SECRET_BYTES {
        bail!(
            "session secret must be at least {MIN_SESSION_SECRET_BYTES} bytes \
             (IDENTITY_SESSION_SECRET / RIDSER_SESSION_SECRET)"
        );
    }
    Ok(())
}

/// Directory for static files and SPAs (default `files`, or `/files` when present).
pub fn files_dir() -> String {
    if let Some(dir) = var_optional("FILES_DIR") {
        return dir;
    }
    if std::path::Path::new("/files").is_dir() {
        return "/files".to_string();
    }
    "files".to_string()
}
