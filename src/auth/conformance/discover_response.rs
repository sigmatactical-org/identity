//! [`DiscoverResponse`].

use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct DiscoverResponse {
    pub(crate) issuer: String,
    pub(crate) status: &'static str,
}
