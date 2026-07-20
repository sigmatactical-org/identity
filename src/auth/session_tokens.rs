//! [`SessionTokens`].

use std::time::SystemTime;

use openidconnect::{AccessToken, RefreshToken, core::CoreIdToken};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::session::SESSION_KEY_JWT;

use super::{
    display_name_from_id_token, email_from_id_token, jwt_payload, token_has_any_realm_role,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SessionTokens {
    access_token: String,
    refresh_token: Option<String>,
    pub(crate) id_token: String,
    pub(crate) expires_at: SystemTime,
    pub(crate) refresh_expires_at: SystemTime,
}

impl SessionTokens {
    /// Token set as stored in the session.
    pub(crate) fn new(
        access_token: &AccessToken,
        refresh_token: Option<&RefreshToken>,
        id_token: &CoreIdToken,
        expires_at: SystemTime,
        refresh_expires_at: SystemTime,
    ) -> Self {
        Self {
            access_token: access_token.secret().to_string(),
            refresh_token: refresh_token.map(|r| r.secret().to_string()),
            id_token: id_token.to_string(),
            expires_at,
            refresh_expires_at,
        }
    }

    /// The bearer access token.
    pub(crate) fn access_token(&self) -> &str {
        &self.access_token
    }

    /// The refresh token, if the IdP issued one.
    pub(crate) fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    /// Whether the access token lives longer than `threshold` seconds.
    pub(crate) fn ttl_gt(&self, threshold: u64) -> bool {
        let now = SystemTime::now();
        self.expires_at
            .duration_since(now)
            .map(|st| st.as_secs())
            .unwrap_or_default()
            > threshold
    }

    /// Display name from the stored ID token (preferred_username, name, or email).
    pub(crate) fn display_name(&self) -> Option<String> {
        display_name_from_id_token(&self.id_token)
    }

    /// Email from the stored ID token.
    pub(crate) fn email(&self) -> Option<String> {
        email_from_id_token(&self.id_token)
    }

    /// Subject (`sub`) from the stored ID token.
    pub(crate) fn subject(&self) -> Option<String> {
        jwt_payload(&self.id_token)?
            .get("sub")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    /// Whether the stored tokens include any of the given realm roles.
    ///
    /// Keycloak's default `roles` client scope puts `realm_access` on the access
    /// token, not the ID token, so both are checked.
    pub(crate) fn has_any_realm_role(&self, roles: &[&str]) -> bool {
        token_has_any_realm_role(&self.access_token, roles)
            || token_has_any_realm_role(&self.id_token, roles)
    }
}

/// The token set stored in the current session, if any.
pub(crate) async fn session_tokens(session: &Session) -> Option<SessionTokens> {
    session.get(SESSION_KEY_JWT).await.ok().flatten()
}
