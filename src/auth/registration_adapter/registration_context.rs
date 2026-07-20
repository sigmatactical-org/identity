//! [`RegistrationContext`].

/// Context passed to registration adapters after a Keycloak user is created.
pub(crate) struct RegistrationContext {
    pub verification_redirect_uri: String,
}
