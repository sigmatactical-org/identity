//! [`LoginAppSettings`].

use axum::http::HeaderValue;
use tracing::trace;

use crate::auth::allowlist::{UriAllowlist, browser_origins_from_entries};

#[derive(Clone, Debug)]
pub struct LoginAppSettings {
    app_uris: UriAllowlist,
    redirect_uris: UriAllowlist,
    cors_origins: Vec<HeaderValue>,
}

impl LoginAppSettings {
    /// Login settings from app/redirect allowlist entries.
    pub fn new(app_uri_entries: Vec<String>, redirect_uri_entries: Vec<String>) -> Self {
        Self {
            cors_origins: browser_origins_from_entries(&app_uri_entries),
            app_uris: UriAllowlist::new(app_uri_entries),
            redirect_uris: UriAllowlist::new(redirect_uri_entries),
        }
    }

    /// Origins allowed for CORS on the login endpoints.
    pub(crate) fn cors_origins(&self) -> &[HeaderValue] {
        &self.cors_origins
    }

    /// Whether an app URI may start a login.
    pub(crate) fn is_app_uri_allowed(&self, app_uri: &str) -> bool {
        trace!("login app_uri check: {app_uri}");
        self.app_uris.is_allowed(app_uri)
    }

    /// Whether a post-login redirect target is allowed.
    pub(crate) fn is_redirect_uri_allowed(&self, redirect_uri: &str) -> bool {
        trace!("login redirect_uri check: {redirect_uri}");
        self.redirect_uris.is_allowed(redirect_uri)
    }
}
