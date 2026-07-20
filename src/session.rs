mod same_site_setting;
mod session_setup;
pub(crate) use same_site_setting::SameSiteSetting;
pub(crate) use session_setup::SessionSetup;

use anyhow::{Context, Result};
use sqlx::PgPool;
use tower_sessions::Session;
use tower_sessions_sqlx_store::PostgresStore;
use tracing::debug;

/// Session slot: CSRF token.
pub(crate) static SESSION_KEY_CSRF_TOKEN: &str = "identity_csrf_token";
/// Session slot: serialized token set.
pub(crate) static SESSION_KEY_JWT: &str = "identity_jwt";
/// Session slot: authenticated user id.
pub(crate) static SESSION_KEY_USERID: &str = "identity_userid";
/// Session slot: pending login callback.
pub(crate) static SESSION_KEY_LOGIN_CALLBACK: &str = "identity_logincallback_parameters";
/// Session slot: app URI that initiated logout.
pub(crate) static SESSION_KEY_LOGOUT_APP_URI: &str = "identity_logout_app_uri";
/// Short-lived cookie carrying post-logout app redirect through the IdP round-trip.
pub(crate) static LOGOUT_APP_URI_COOKIE: &str = "identity.logout.app_uri";

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
