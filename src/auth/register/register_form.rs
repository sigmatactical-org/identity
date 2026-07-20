//! [`RegisterForm`].

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterForm {
    pub(crate) return_url: String,
    pub(crate) email: String,
    pub(crate) username: String,
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    pub(crate) password: String,
    pub(crate) password_confirm: String,
    #[serde(default)]
    pub(crate) altcha: String,
}
