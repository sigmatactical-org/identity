//! [`CreateUserPayload`].

#[allow(unused_imports)]
use super::*;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateUserPayload<'a> {
    pub(crate) username: &'a str,
    pub(crate) email: &'a str,
    pub(crate) first_name: &'a str,
    pub(crate) last_name: &'a str,
    pub(crate) enabled: bool,
    pub(crate) email_verified: bool,
    pub(crate) required_actions: [&'static str; 1],
    pub(crate) attributes: HashMap<&'static str, Vec<String>>,
    pub(crate) credentials: [PasswordCredential<'a>; 1],
}
