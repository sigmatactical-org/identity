//! [`IntrospectionResponse`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct IntrospectionResponse {
    #[serde(default)]
    pub(crate) active: bool,
    #[serde(default)]
    pub(crate) realm_access: Option<RealmAccessClaim>,
}
