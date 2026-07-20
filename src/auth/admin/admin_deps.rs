//! [`AdminDeps`].

use crate::auth::KeycloakAdmin;

#[derive(Clone)]
pub(crate) struct AdminDeps {
    pub admin: KeycloakAdmin,
}
