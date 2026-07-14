//! [`OidcInner`].

#[allow(unused_imports)]
use super::*;
use openidconnect::core::CoreProviderMetadata;

#[derive(Clone)]
pub(crate) struct OidcInner {
    pub(crate) client: OidcCoreClient,
    pub(crate) metadata: CoreProviderMetadata,
}
