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

/// Listen port: `IDENTITY_BIND_PORT`, then Cloud Run `PORT`, then default `3000`.
pub fn listen_port() -> String {
    var_optional("BIND_PORT")
        .or_else(|| std::env::var("PORT").ok())
        .unwrap_or_else(|| String::from("3000"))
}

/// PostgreSQL connection URL for sessions and health checks.
pub fn database_url() -> String {
    var_optional("DATABASE_URL").unwrap_or_else(|| sigma_pg::DEFAULT_DATABASE_URL.to_string())
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

/// When true, OIDC conformance testing assets and relaxed login defaults are enabled.
pub fn conformance_mode() -> bool {
    matches!(
        var_optional("CONFORMANCE_MODE").as_deref(),
        Some("1") | Some("true") | Some("TRUE")
    )
}

/// When set, discover OIDC metadata from this issuer URL (cluster-internal) instead of
/// `IDENTITY_OIDC_ISSUER_URL` (public hostname browsers and ID tokens use).
pub fn oidc_discovery_issuer_url() -> Option<String> {
    var_optional("OIDC_DISCOVERY_ISSUER_URL")
}

/// When set, override the token endpoint from discovery (cluster-internal backchannel).
pub fn oidc_token_endpoint() -> Option<String> {
    var_optional("OIDC_TOKEN_ENDPOINT")
}

/// When set, override the JWKS URI from discovery (cluster-internal backchannel).
pub fn oidc_jwks_uri() -> Option<String> {
    var_optional("OIDC_JWKS_URI")
}

/// When set: `client_secret_basic` or `client_secret_post` for token endpoint auth.
pub fn oidc_token_endpoint_auth_method() -> Option<String> {
    var_optional("OIDC_TOKEN_ENDPOINT_AUTH_METHOD").map(|v| v.trim().to_ascii_lowercase())
}

/// `header` (default) or `body` for UserInfo access token presentation.
pub fn oidc_userinfo_method() -> String {
    var_optional("OIDC_USERINFO_METHOD")
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "header".to_string())
}

/// When set, discover the issuer via WebFinger for this resource (acct: or https: URL).
pub fn oidc_webfinger_resource() -> Option<String> {
    var_optional("OIDC_WEBFINGER_RESOURCE")
}

/// When true, refetch JWKS after UserInfo (rotation certification modules).
pub fn oidc_jwks_refresh_after_userinfo() -> bool {
    matches!(
        var_optional("OIDC_JWKS_REFRESH_AFTER_USERINFO").as_deref(),
        Some("1") | Some("true") | Some("TRUE")
    )
}

/// When set to `form_post`, request authorization responses via POST to redirect_uri.
pub fn oidc_authorize_response_mode() -> Option<String> {
    var_optional("OIDC_AUTHORIZE_RESPONSE_MODE").map(|v| v.trim().to_ascii_lowercase())
}

/// When true, include id_token_hint on RP-initiated logout (conformance / Keycloak).
pub fn logout_send_id_token_hint() -> bool {
    matches!(
        var_optional("LOGOUT_SEND_ID_TOKEN_HINT").as_deref(),
        Some("1") | Some("true") | Some("TRUE")
    )
}

/// When false, the self-service registration page is not mounted.
pub fn registration_enabled() -> bool {
    !conformance_mode()
        && !matches!(
            var_optional("REGISTRATION_DISABLED").as_deref(),
            Some("1") | Some("true") | Some("TRUE")
        )
}

/// Allowed `return_url` destinations for `/register` (comma-separated, exact or trailing `*`).
pub fn registration_return_uris() -> Result<Vec<String>> {
    let raw = if let Some(value) = var_optional("REGISTRATION_RETURN_URIS") {
        value
    } else {
        var("LOGIN_REDIRECT_APP_URIS")?
    };
    Ok(raw
        .split(',')
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect())
}
