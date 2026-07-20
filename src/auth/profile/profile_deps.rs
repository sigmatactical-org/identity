//! [`ProfileDeps`].

use crate::auth::{KeycloakAdmin, RegistrationAppSettings};

#[derive(Clone)]
pub(crate) struct ProfileDeps {
    pub settings: RegistrationAppSettings,
    pub admin: KeycloakAdmin,
}
