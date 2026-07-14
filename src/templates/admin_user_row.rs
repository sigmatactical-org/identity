//! [`AdminUserRow`].

#[allow(unused_imports)]
use super::*;

/// One user in the admin list.
pub(crate) struct AdminUserRow {
    pub id: String,
    pub username: String,
    pub email: String,
    pub name: String,
    pub enabled: bool,
    pub email_verified: bool,
    pub created: String,
}
