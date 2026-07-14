//! [`AdminServiceAccountRow`].

#[allow(unused_imports)]
use super::*;

/// A client's service-account user, for the admin "Service accounts" table.
pub(crate) struct AdminServiceAccountRow {
    pub id: String,
    pub client_id: String,
    pub username: String,
    pub enabled: bool,
    pub created: String,
}
