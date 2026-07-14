//! [`UserSummary`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

/// Summary row for the admin user list (`GET /admin/realms/.../users`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserSummary {
    pub id: String,
    pub username: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub enabled: bool,
    pub email_verified: bool,
    pub created_timestamp: Option<i64>,
}
