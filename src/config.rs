//! Environment configuration. Prefer `IDENTITY_*` variables; `RIDSER_*` is accepted as a
//! deprecated fallback for upstream compatibility.
//!
//! Peer service URLs use [`sigma_config::service!`] (fail-fast when
//! `SIGMA_DEV_DEFAULTS` is unset). OIDC/session secrets keep the dual-prefix
//! [`var`] / [`var_optional`] path because they are not base URLs and still
//! accept deprecated `RIDSER_*` names.

mod registration_mode;
pub use registration_mode::RegistrationMode;

use std::env;

use anyhow::{Context, Result, bail};

use crate::session::SameSiteSetting;

const MIN_SESSION_SECRET_BYTES: usize = 32;

sigma_config::service! {
    prefix = "IDENTITY";
    role = "identity";
    urls {
        /// Public base URL of the cart service for navbar links.
        cart_public_base_url = "CART_PUBLIC_URL" => "http://127.0.0.1:8084/";
        /// Public base URL of the contact service for navbar links.
        contact_public_base_url = "CONTACT_PUBLIC_URL" => "http://127.0.0.1:8083/";
        /// Public base URL of the addresses service (profile account links).
        addresses_public_base_url = "ADDRESSES_PUBLIC_URL" => "http://127.0.0.1:8089/";
        /// Public base URL of the payments service (profile account links).
        payments_public_base_url = "PAYMENTS_PUBLIC_URL" => "http://127.0.0.1:8090/";
        /// Public base URL of the orders service (profile account links).
        orders_public_base_url = "ORDERS_PUBLIC_URL" => "http://127.0.0.1:8085/";
    }
}

/// Read `IDENTITY_{name}` or fall back to deprecated `RIDSER_{name}`.
pub fn var(name: &str) -> Result<String> {
    let identity = format!("IDENTITY_{name}");
    if let Ok(v) = env::var(&identity) {
        return Ok(v);
    }
    let legacy = format!("RIDSER_{name}");
    env::var(&legacy).with_context(|| format!("missing {identity} (or deprecated {legacy})"))
}

/// Env var as `Some` only when set and non-empty.
pub fn var_optional(name: &str) -> Option<String> {
    let identity = format!("IDENTITY_{name}");
    env::var(&identity)
        .ok()
        .or_else(|| env::var(format!("RIDSER_{name}")).ok())
}

/// Comma-separated list from a required env var (entries trimmed, empties dropped).
pub fn var_list(name: &str) -> Result<Vec<String>> {
    Ok(split_list(&var(name)?))
}

fn split_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(str::to_string)
        .collect()
}

/// Whether `SESSION_SECURE_COOKIE_DISABLED` turns off the Secure cookie flag.
pub fn session_secure_cookie_disabled() -> bool {
    var_optional("SESSION_SECURE_COOKIE_DISABLED").is_some_and(|v| v.eq_ignore_ascii_case("true"))
}

/// SameSite mode for session-adjacent cookies (`SESSION_COOKIE_SAMESITE`).
pub fn session_cookie_same_site() -> SameSiteSetting {
    SameSiteSetting::from_env_string(var_optional("SESSION_COOKIE_SAMESITE"))
}

/// PostgreSQL URL for integration tests (`TEST_DATABASE_URL` or `IDENTITY_TEST_DATABASE_URL`).
#[cfg(test)]
pub fn test_database_url() -> String {
    env::var("TEST_DATABASE_URL")
        .ok()
        .or_else(|| var_optional("TEST_DATABASE_URL"))
        .unwrap_or_else(|| sigma_pg::DEFAULT_DATABASE_URL.to_string())
}

/// Listen port: `IDENTITY_BIND_PORT`, then Cloud Run `PORT`, then default `3000`.
pub fn listen_port() -> String {
    var_optional("BIND_PORT")
        .or_else(|| std::env::var("PORT").ok())
        .unwrap_or_else(|| String::from("3000"))
}

/// PostgreSQL connection URL for sessions and health checks.
///
/// Identity uses the prefixed `IDENTITY_DATABASE_URL` (with deprecated
/// `RIDSER_DATABASE_URL` fallback), not the unprefixed `DATABASE_URL` shared by
/// the other services.
pub fn database_url() -> String {
    var_optional("DATABASE_URL").unwrap_or_else(|| sigma_pg::service_database_url("identity"))
}

/// Whether `SIGMA_ENV` marks this deployment as production.
pub fn is_production() -> bool {
    matches!(
        var_optional("ENV")
            .or_else(|| env::var("RUST_ENV").ok())
            .as_deref(),
        Some("production" | "prod")
    )
}

/// Dev-only TLS bypass flag; refused in production.
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

/// Require a session secret long enough for cookie signing.
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

/// When set, Keycloak Admin API and password-grant backchannel calls use this base URL
/// (e.g. `http://keycloak.sigma-dev.svc.cluster.local`) instead of the public issuer host.
pub fn keycloak_admin_base_url() -> Option<String> {
    var_optional("KEYCLOAK_ADMIN_BASE_URL").map(|v| v.trim_end_matches('/').to_string())
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

/// Registration mode, selected solely by `REGISTRATION_MODE` so it is
/// independent of the production hardening flag (`is_production`, which drives
/// security headers). An explicit value is honored in every environment; the
/// default is the safe `VerifyEmail` in production and `AutoApprove` only in
/// non-production, so local dev keeps its fast path without an email server
/// while a prod deploy never silently auto-enables accounts.
pub fn registration_mode() -> RegistrationMode {
    match var_optional("REGISTRATION_MODE").as_deref().map(str::trim) {
        Some("verify_email" | "email" | "verification") => RegistrationMode::VerifyEmail,
        Some("auto_approve" | "approve" | "auto") => RegistrationMode::AutoApprove,
        _ if is_production() => RegistrationMode::VerifyEmail,
        _ => RegistrationMode::AutoApprove,
    }
}

/// Public browser base URL for links in verification emails (no trailing slash).
pub fn public_base_url() -> String {
    var_optional("PUBLIC_BASE_URL")
        .filter(|value| !value.trim().is_empty())
        .map(|value| value.trim_end_matches('/').to_string())
        .unwrap_or_else(|| "http://127.0.0.1:3000".to_string())
}

/// Allowed `return_url` destinations for `/register` (comma-separated, exact or trailing `*`).
pub fn registration_return_uris() -> Result<Vec<String>> {
    let raw = if let Some(value) = var_optional("REGISTRATION_RETURN_URIS") {
        value
    } else {
        var("LOGIN_REDIRECT_APP_URIS")?
    };
    Ok(split_list(&raw))
}

/// Realm roles that grant access to `/admin/*` (comma-separated). Defaults to
/// `sigma-admin`.
pub fn admin_realm_roles() -> Vec<String> {
    split_list(&var_optional("ADMIN_REALM_ROLES").unwrap_or_else(|| "sigma-admin".to_string()))
}
