//! [`ProfileQueryParams`].

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct ProfileQueryParams {
    #[serde(default)]
    pub(crate) return_url: String,
    #[serde(default)]
    pub(crate) edit: Option<String>,
}
