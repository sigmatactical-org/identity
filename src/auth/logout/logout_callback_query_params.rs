//! [`LogoutCallbackQueryParams`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LogoutCallbackQueryParams {
    #[serde(rename = "app_uri")]
    pub(crate) app_uri: Option<String>,
}
