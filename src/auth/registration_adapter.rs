use anyhow::Result;

use super::keycloak_admin::{CreatedUser, KeycloakAdmin};

/// Context passed to registration adapters after a Keycloak user is created.
pub(crate) struct RegistrationContext {
    pub verification_redirect_uri: String,
}

/// Result of adapter-specific post-registration handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistrationOutcome {
    /// Account is enabled and ready to sign in (dev auto-approve).
    Approved,
    /// Account awaits email verification.
    PendingEmailVerification,
}

/// Post-registration policy selected by environment / config.
pub(crate) trait AccountRegistrationAdapter: Send + Sync {
    fn finalize_registration(
        &self,
        admin: &KeycloakAdmin,
        user: &CreatedUser,
        context: &RegistrationContext,
    ) -> impl std::future::Future<Output = Result<RegistrationOutcome>> + Send;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DevAccountRegistrationAdapter;

impl AccountRegistrationAdapter for DevAccountRegistrationAdapter {
    async fn finalize_registration(
        &self,
        admin: &KeycloakAdmin,
        user: &CreatedUser,
        _context: &RegistrationContext,
    ) -> Result<RegistrationOutcome> {
        admin.approve_user(&user.id).await?;
        Ok(RegistrationOutcome::Approved)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct EmailVerificationAccountRegistrationAdapter;

impl AccountRegistrationAdapter for EmailVerificationAccountRegistrationAdapter {
    async fn finalize_registration(
        &self,
        admin: &KeycloakAdmin,
        user: &CreatedUser,
        context: &RegistrationContext,
    ) -> Result<RegistrationOutcome> {
        admin
            .send_verification_email_while_disabled(&user.id, &context.verification_redirect_uri)
            .await?;
        Ok(RegistrationOutcome::PendingEmailVerification)
    }
}

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
