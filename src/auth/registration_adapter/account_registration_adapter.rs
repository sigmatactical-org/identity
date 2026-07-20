//! [`AccountRegistrationAdapter`].

use anyhow::Result;

use super::{RegistrationContext, RegistrationOutcome};
use crate::auth::keycloak_admin::{CreatedUser, KeycloakAdmin};

/// Post-registration policy selected by environment / config.
pub(crate) trait AccountRegistrationAdapter: Send + Sync {
    fn finalize_registration(
        &self,
        admin: &KeycloakAdmin,
        user: &CreatedUser,
        context: &RegistrationContext,
    ) -> impl std::future::Future<Output = Result<RegistrationOutcome>> + Send;
}
