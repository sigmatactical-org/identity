//! [`UpdateProfilePayload`].

#[allow(unused_imports)]
use super::*;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateProfilePayload<'a> {
    pub(crate) username: &'a str,
    pub(crate) email: &'a str,
    pub(crate) first_name: &'a str,
    pub(crate) last_name: &'a str,
    pub(crate) attributes: HashMap<String, Vec<String>>,
}
