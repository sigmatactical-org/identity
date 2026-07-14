//! [`ApproveUserPayload`].

#[allow(unused_imports)]
use super::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApproveUserPayload {
    pub(crate) enabled: bool,
    pub(crate) email_verified: bool,
    pub(crate) required_actions: Vec<&'static str>,
}
