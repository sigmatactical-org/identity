//! [`LoginCallbackSessionParameters`].

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LoginCallbackSessionParameters {
    pub(crate) app_uri: String,
    pub(crate) nonce: String,
    pub(crate) csrf_token: String,
    pub(crate) pkce_verifier: String,
    pub(crate) redirect_uri: String,
    pub(crate) scopes: String,
}
