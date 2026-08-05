//! [`StatusResponse`].

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StatusResponse {
    pub(crate) expires_in: Option<u64>,
    pub(crate) refresh_expires_in: Option<u64>,
    pub(crate) authenticated: bool,
    /// Whether the session holds an admin realm role (`IDENTITY_ADMIN_REALM_ROLES`).
    /// Consumed by peer services (e.g. orders) that gate admin-only HTML surfaces.
    #[serde(default)]
    pub(crate) is_admin: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) user_id: Option<String>,
}
