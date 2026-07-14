//! [`LogoutQueryParams`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LogoutQueryParams {
    #[serde(rename = "app_uri")]
    pub(crate) app_uri: String,
    #[serde(rename = "redirect_uri")]
    pub(crate) redirect_uri: String,
}
