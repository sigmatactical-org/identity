//! [`TokenResponse`].

use serde::Deserialize;

/// Keycloak's client-credentials token response.
///
/// `sigma_pg::clients::identity` has an equivalent type, but it is private to
/// that crate, so this stays local.
#[derive(Debug, Deserialize)]
pub(crate) struct TokenResponse {
    pub(crate) access_token: String,
    /// Token lifetime in seconds; Keycloak always sends it.
    #[serde(default = "default_expires_in")]
    pub(crate) expires_in: u64,
}

/// Keycloak's own default access-token lifespan, used when the field is absent.
fn default_expires_in() -> u64 {
    60
}
