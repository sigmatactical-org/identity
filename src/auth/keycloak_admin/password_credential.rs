//! [`PasswordCredential`].

#[allow(unused_imports)]
use super::*;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct PasswordCredential<'a> {
    #[serde(rename = "type")]
    pub(crate) credential_type: &'static str,
    pub(crate) value: &'a str,
    pub(crate) temporary: bool,
}
