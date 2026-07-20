//! [`RegisterSuccessQuery`].

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterSuccessQuery {
    pub(crate) return_url: String,
    #[serde(default)]
    pub(crate) approved: Option<bool>,
}
