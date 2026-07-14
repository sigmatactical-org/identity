//! [`ClientSummary`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClientSummary {
    pub(crate) id: String,
    pub(crate) client_id: String,
    #[serde(default)]
    pub(crate) service_accounts_enabled: bool,
}
