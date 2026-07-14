//! [`LogoutAppSettings`].

use super::super::allowlist::UriAllowlist;
#[allow(unused_imports)]
use super::*;
use tracing::trace;

#[derive(Clone, Debug)]
pub struct LogoutAppSettings {
    pub(crate) client_id: String,
    pub(crate) logout_uri: String,
    pub(crate) _behavior: LogoutBehavior,
    pub(crate) allowed_app_uris: UriAllowlist,
    pub(crate) allowed_oidc_redirect_uris: UriAllowlist,
}
impl LogoutAppSettings {
    /// Whether an app URI may start a logout.
    pub(crate) fn is_app_uri_allowed(&self, app_uri: &str) -> bool {
        trace!("logout app_uri check: {app_uri}");
        self.allowed_app_uris.is_allowed(app_uri)
    }

    /// Whether a post-logout redirect target is allowed.
    pub(crate) fn is_oidc_redirect_allowed(&self, redirect_uri: &str) -> bool {
        trace!("logout oidc redirect check: {redirect_uri}");
        self.allowed_oidc_redirect_uris.is_allowed(redirect_uri)
    }
}
