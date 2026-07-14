//! [`RegisterVerifiedQuery`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterVerifiedQuery {
    #[serde(default)]
    pub(crate) return_url: Option<String>,
}
