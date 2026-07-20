//! [`AuthorizeData`].

use openidconnect::{CsrfToken, Nonce, PkceCodeVerifier, url::Url};

#[derive(Debug, Clone)]
pub(crate) struct AuthorizeData {
    pub(crate) auth_url: String,
    pub(crate) csrf_token: String,
    pub(crate) nonce: String,
    pub(crate) pkce_verifier: String,
}

impl AuthorizeData {
    /// Bundle the parameters of one in-flight authorization request.
    pub(crate) fn new(
        auth_url: Url,
        csrf_token: CsrfToken,
        nonce: Nonce,
        pkce_verifier: PkceCodeVerifier,
    ) -> Self {
        Self {
            auth_url: auth_url.to_string(),
            csrf_token: csrf_token.secret().clone(),
            nonce: nonce.secret().clone(),
            pkce_verifier: pkce_verifier.secret().to_string(),
        }
    }
}
