//! [`EmailVerificationAccountRegistrationAdapter`].

use anyhow::Result;

use super::{AccountRegistrationAdapter, RegistrationContext, RegistrationOutcome};
use crate::auth::keycloak_admin::{CreatedUser, KeycloakAdmin};

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
