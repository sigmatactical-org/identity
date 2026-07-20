//! [`OidcInner`].

use std::time::Instant;

use openidconnect::core::{CoreJsonWebKeySet, CoreProviderMetadata};
use tokio::sync::RwLock;

use super::OidcCoreClient;

pub(crate) struct OidcInner {
    pub(crate) client: OidcCoreClient,
    pub(crate) metadata: CoreProviderMetadata,
    /// Cached JWKS with its fetch time; the TTL is applied by
    /// [`super::OIDCClient::jwks`], which also refreshes on demand.
    pub(crate) jwks: RwLock<Option<(Instant, CoreJsonWebKeySet)>>,
}

impl OidcInner {
    pub(crate) fn new(client: OidcCoreClient, metadata: CoreProviderMetadata) -> Self {
        Self {
            client,
            metadata,
            jwks: RwLock::new(None),
        }
    }
}
