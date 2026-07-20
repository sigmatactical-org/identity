//! [`UpdateUserPayload`].

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateUserPayload {
    pub(crate) enabled: bool,
}
