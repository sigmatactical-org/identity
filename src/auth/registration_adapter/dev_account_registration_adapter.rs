//! [`DevAccountRegistrationAdapter`].

use anyhow::Result;

use super::{AccountRegistrationAdapter, RegistrationContext, RegistrationOutcome};
use crate::auth::keycloak_admin::{CreatedUser, KeycloakAdmin};

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
