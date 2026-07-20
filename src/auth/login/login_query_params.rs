//! [`LoginQueryParams`].

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct LoginQueryParams {
    pub(crate) app_uri: String,
    pub(crate) redirect_uri: String,
    pub(crate) scope: String,
    pub(crate) ui_locales: Option<String>,
    pub(crate) prompt: Option<String>,
    pub(crate) kc_idp_hint: Option<String>,
}
