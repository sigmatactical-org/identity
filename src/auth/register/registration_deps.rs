//! [`RegistrationDeps`].

use super::RegistrationAppSettings;

use super::super::{keycloak_admin::KeycloakAdmin, registration_adapter::RegistrationAdapter};

#[derive(Clone)]
pub struct RegistrationDeps {
    pub settings: RegistrationAppSettings,
    pub admin: KeycloakAdmin,
    pub adapter: RegistrationAdapter,
}
