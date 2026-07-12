use time::Duration;

use anyhow::{Context, Result};
use cookie::Key;
use sqlx::PgPool;
use tower_sessions::{Expiry, Session, SessionManagerLayer, SessionStore, service::PrivateCookie};
use tower_sessions_sqlx_store::PostgresStore;
use tracing::debug;

pub(crate) static SESSION_KEY_CSRF_TOKEN: &str = "identity_csrf_token";
pub(crate) static SESSION_KEY_JWT: &str = "identity_jwt";
pub(crate) static SESSION_KEY_USERID: &str = "identity_userid";
pub(crate) static SESSION_KEY_LOGIN_CALLBACK: &str = "identity_logincallback_parameters";
pub(crate) static SESSION_KEY_LOGOUT_APP_URI: &str = "identity_logout_app_uri";
/// Short-lived cookie carrying post-logout app redirect through the IdP round-trip.
pub(crate) static LOGOUT_APP_URI_COOKIE: &str = "identity.logout.app_uri";

#[derive(Debug, Clone)]
pub(crate) enum SameSiteSetting {
    None,
    Lax,
    Strict,
}

impl SameSiteSetting {
    pub(crate) fn from_env_string(value: Option<String>) -> Self {
        match value.as_deref().map(|s| s.to_lowercase()).as_deref() {
            Some("none") => SameSiteSetting::None,
            Some("lax") => SameSiteSetting::Lax,
            Some("strict") => SameSiteSetting::Strict,
            _ => SameSiteSetting::Lax,
        }
    }

    pub(crate) fn to_tower_sessions_same_site(&self) -> tower_sessions::cookie::SameSite {
        match self {
            SameSiteSetting::None => tower_sessions::cookie::SameSite::None,
            SameSiteSetting::Lax => tower_sessions::cookie::SameSite::Lax,
            SameSiteSetting::Strict => tower_sessions::cookie::SameSite::Strict,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SessionSetup {
    pub(crate) secret: String,
    pub(crate) cookie_name: String,
    pub(crate) cookie_path: String,
    pub(crate) cookie_domain: Option<String>,
    pub(crate) ttl: Option<Duration>,
    pub(crate) secure_cookie: bool,
    pub(crate) same_site: SameSiteSetting,
}

impl SessionSetup {
    pub(crate) fn get_session_layer<Store: SessionStore>(
        &self,
        store: Store,
    ) -> Result<SessionManagerLayer<Store, PrivateCookie>> {
        debug!("Preparing session");
        let mut session_layer = SessionManagerLayer::new(store)
            .with_private(Key::from(self.secret.as_bytes()))
            .with_name(self.cookie_name.clone())
            .with_secure(self.secure_cookie)
            .with_path(self.cookie_path.clone())
            .with_same_site(self.same_site.to_tower_sessions_same_site())
            .with_expiry(Expiry::OnInactivity(
                self.ttl.unwrap_or_else(|| Duration::hours(1)),
            ));
        if let Some(domain) = self.cookie_domain.clone() {
            session_layer = session_layer.with_domain(domain);
        }

        Ok(session_layer)
    }
}

pub(crate) async fn postgres_pool(connection_url: &str) -> Result<PgPool> {
    debug!("Establishing PostgreSQL session connection");
    let pool = PgPool::connect(connection_url)
        .await
        .with_context(|| format!("connect to PostgreSQL at {connection_url}"))?;
    Ok(pool)
}

pub(crate) async fn session_store(pool: PgPool) -> Result<PostgresStore> {
    let store = PostgresStore::new(pool)
        .with_schema_name("identity")
        .map_err(|e| anyhow::anyhow!("invalid identity session schema: {e}"))?;
    store
        .migrate()
        .await
        .context("migrate tower_sessions tables")?;
    Ok(store)
}

/// Remove session data and start a fresh session identifier.
pub(crate) async fn purge_store_and_regenerate_session(session: &Session) {
    let _: Result<(), _> = session.flush().await;
    let _ = session.save().await;
}
