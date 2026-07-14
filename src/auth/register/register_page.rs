//! [`RegisterPage`].

#[allow(unused_imports)]
use super::*;

/// Query/session inputs for rendering the registration page.
pub(crate) struct RegisterPage {
    pub(crate) return_url: String,
    pub(crate) email: String,
    pub(crate) username: String,
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    pub(crate) error: Option<String>,
    pub(crate) human_check: sigma_human_check::HumanCheck,
}
