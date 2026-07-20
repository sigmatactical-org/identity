//! [`IntrospectionResult`].

/// Result of an RFC 7662 token introspection against the identity provider.
/// `active` reflects the IdP's authoritative validity decision (signature,
/// expiry, revocation); `realm_roles` are the Keycloak realm roles the IdP
/// reports for an active token (empty when inactive).
#[derive(Debug, Clone, Default)]
pub(crate) struct IntrospectionResult {
    pub(crate) active: bool,
    pub(crate) realm_roles: Vec<String>,
}
