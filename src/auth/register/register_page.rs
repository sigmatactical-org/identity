//! [`RegisterPage`].

use super::RegisterForm;

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

impl RegisterPage {
    /// Re-render the submitted form with an error message, preserving input.
    pub(crate) fn from_form(
        form: RegisterForm,
        human_check: sigma_human_check::HumanCheck,
        error: String,
    ) -> Self {
        Self {
            return_url: form.return_url,
            email: form.email,
            username: form.username,
            first_name: form.first_name,
            last_name: form.last_name,
            error: Some(error),
            human_check,
        }
    }
}
