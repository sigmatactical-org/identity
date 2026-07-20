//! [`TokenExchangeData`].

#[derive(Debug, Clone)]
pub(crate) struct TokenExchangeData {
    pub(crate) code: String,
    pub(crate) nonce: String,
    pub(crate) pkce_verifier: String,
    pub(crate) redirect_uri: String,
}
