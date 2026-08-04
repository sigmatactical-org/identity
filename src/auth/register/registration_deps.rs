//! [`RegistrationDeps`].

use super::RegistrationAppSettings;

use super::super::{keycloak_admin::KeycloakAdmin, registration_adapter::RegistrationAdapter};

#[derive(Clone)]
pub struct RegistrationDeps {
    pub settings: RegistrationAppSettings,
    pub admin: KeycloakAdmin,
    pub adapter: RegistrationAdapter,
    /// Bot check guarding `POST /register`, built during startup so a missing
    /// secret stops the service instead of opening the form to scripts.
    pub human_check: sigma_human_check::HumanCheck,
}
