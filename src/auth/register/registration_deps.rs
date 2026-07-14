//! [`RegistrationDeps`].

use super::super::{keycloak_admin::KeycloakAdmin, registration_adapter::RegistrationAdapter};
#[allow(unused_imports)]
use super::*;

#[derive(Clone)]
pub struct RegistrationDeps {
    pub settings: RegistrationAppSettings,
    pub admin: KeycloakAdmin,
    pub adapter: RegistrationAdapter,
}
