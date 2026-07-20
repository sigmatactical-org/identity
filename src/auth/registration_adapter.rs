mod account_registration_adapter;
mod dev_account_registration_adapter;
mod email_verification_account_registration_adapter;
mod registration_context;
mod registration_outcome;
pub(crate) use account_registration_adapter::AccountRegistrationAdapter;
pub(crate) use dev_account_registration_adapter::DevAccountRegistrationAdapter;
pub(crate) use email_verification_account_registration_adapter::EmailVerificationAccountRegistrationAdapter;
pub(crate) use registration_context::RegistrationContext;
pub(crate) use registration_outcome::RegistrationOutcome;

use anyhow::Result;

use super::keycloak_admin::{CreatedUser, KeycloakAdmin};

/// Selects the registration adapter from the registration mode. This is
/// deliberately independent of `is_production` (which only drives security
/// headers): a production deploy runs the email-verification flow via
/// `REGISTRATION_MODE=verify_email`, and hardened headers do not silently
/// disable or change registration.
#[derive(Clone, Copy, Debug)]
pub(crate) enum RegistrationAdapter {
    Dev(DevAccountRegistrationAdapter),
    VerifyEmail(EmailVerificationAccountRegistrationAdapter),
}

impl RegistrationAdapter {
    #[must_use]
    pub fn from_env() -> Self {
        Self::for_mode(crate::config::registration_mode())
    }

    #[must_use]
    fn for_mode(mode: crate::config::RegistrationMode) -> Self {
        match mode {
            crate::config::RegistrationMode::AutoApprove => {
                Self::Dev(DevAccountRegistrationAdapter)
            }
            crate::config::RegistrationMode::VerifyEmail => {
                Self::VerifyEmail(EmailVerificationAccountRegistrationAdapter)
            }
        }
    }

    pub async fn finalize_registration(
        &self,
        admin: &KeycloakAdmin,
        user: &CreatedUser,
        context: &RegistrationContext,
    ) -> Result<RegistrationOutcome> {
        match self {
            Self::Dev(adapter) => adapter.finalize_registration(admin, user, context).await,
            Self::VerifyEmail(adapter) => adapter.finalize_registration(admin, user, context).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registration_adapter_selects_dev_auto_approve() {
        assert!(matches!(
            RegistrationAdapter::for_mode(crate::config::RegistrationMode::AutoApprove),
            RegistrationAdapter::Dev(_)
        ));
    }

    #[test]
    fn registration_adapter_selects_verify_email_mode() {
        assert!(matches!(
            RegistrationAdapter::for_mode(crate::config::RegistrationMode::VerifyEmail),
            RegistrationAdapter::VerifyEmail(_)
        ));
    }
}
