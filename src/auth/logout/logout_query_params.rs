//! [`LogoutQueryParams`].

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LogoutQueryParams {
    pub(crate) app_uri: String,
    pub(crate) redirect_uri: String,
}
