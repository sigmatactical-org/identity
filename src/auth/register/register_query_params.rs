//! [`RegisterQueryParams`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterQueryParams {
    pub(crate) return_url: String,
    #[serde(default)]
    pub(crate) email: Option<String>,
    #[serde(default, alias = "given_name")]
    pub(crate) first_name: Option<String>,
    #[serde(default, alias = "family_name")]
    pub(crate) last_name: Option<String>,
    #[serde(default)]
    pub(crate) username: Option<String>,
}
