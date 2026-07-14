//! [`AuthorizeRequestData`].

#[allow(unused_imports)]
use super::*;

/// Parameters carried through the authorization-code flow.
pub struct AuthorizeRequestData {
    pub redirect_uri: String,
    pub scope: String,
    pub state: String,
    pub ui_locales: Option<String>,
    pub prompt: Option<String>,
    pub kc_idp_hint: Option<String>,
    /// Passed to Keycloak as `sigma_return_url` for the login-page register link.
    pub register_return_url: Option<String>,
}
