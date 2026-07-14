//! [`UserRepresentation`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserRepresentation {
    pub(crate) username: Option<String>,
    pub(crate) email: Option<String>,
    pub(crate) first_name: Option<String>,
    pub(crate) last_name: Option<String>,
    pub(crate) enabled: bool,
    pub(crate) email_verified: bool,
    pub(crate) created_timestamp: Option<i64>,
    pub(crate) attributes: Option<HashMap<String, Vec<String>>>,
}
