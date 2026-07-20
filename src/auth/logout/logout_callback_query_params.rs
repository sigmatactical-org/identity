//! [`LogoutCallbackQueryParams`].

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LogoutCallbackQueryParams {
    pub(crate) app_uri: Option<String>,
}
