#![forbid(unsafe_code)]

use std::env;

use anyhow::Context;
use session::SessionSetup;
use tokio::signal;
use tracing::{debug, warn};

use crate::{
    auth::{
        AdminDeps, AppConfigurationState, KeycloakAdmin, LoginAppSettings, LogoutAppSettings,
        OIDCClient, ProfileDeps, RegistrationAdapter, RegistrationAppSettings, RegistrationDeps,
        allowlist::UriAllowlist,
    },
    config::validate_session_secret,
    http::{ProxyConfig, app},
    session::{postgres_pool, session_store},
};

mod auth;
mod config;
mod geo;
mod http;
mod monitoring;
mod session;
mod site_nav;
mod templates;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install shutdown handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    debug!("signal received, starting graceful shutdown");
}

async fn init_oidc_client(client_id: &str) -> anyhow::Result<OIDCClient> {
    let issuer_url = crate::config::var("OIDC_ISSUER_URL")?;
    let client_secret = crate::config::var("OIDC_CLIENT_SECRET")?;
    let auth_url = crate::config::var_optional("OIDC_AUTH_URL");
    OIDCClient::build(
        &issuer_url,
        client_id,
        &client_secret,
        auth_url,
        crate::config::oidc_discovery_issuer_url(),
        crate::config::oidc_token_endpoint(),
        crate::config::oidc_jwks_uri(),
    )
    .await
}

fn init_session_vars() -> anyhow::Result<SessionSetup> {
    let secure_disabled = crate::config::session_secure_cookie_disabled();
    if secure_disabled {
        warn!("Disabling secure cookies is not recommended in production.");
    }

    let same_site = crate::config::session_cookie_same_site();

    let secret = crate::config::var("SESSION_SECRET")?;
    validate_session_secret(&secret)?;

    Ok(SessionSetup {
        cookie_name: crate::config::var_optional("SESSION_COOKIE_NAME")
            .unwrap_or_else(|| "identity.sid".to_string()),
        cookie_path: crate::config::var_optional("SESSION_COOKIE_PATH")
            .unwrap_or_else(|| "/".to_string()),
        cookie_domain: crate::config::var_optional("SESSION_COOKIE_DOMAIN"),
        secret,
        ttl: None,
        secure_cookie: !secure_disabled,
        same_site,
    })
}

pub async fn run() -> anyhow::Result<()> {
    let session_setup = init_session_vars()?;
    let database_url = config::database_url();
    let pool = postgres_pool(&database_url).await?;
    let store = session_store(pool.clone()).await?;
    let session_layer = session_setup.get_session_layer(store)?;
    let client_id = crate::config::var("OIDC_CLIENT_ID")?;
    let oidc_client = init_oidc_client(&client_id).await?;
    if config::conformance_mode() {
        debug!("OIDC conformance mode enabled (see /conformance/ harness)");
    }
    let proxy_rules: Vec<_> = env::vars()
        .filter(|(key, value)| {
            (key.starts_with("IDENTITY_PROXY_TARGET_RULE_")
                || key.starts_with("RIDSER_PROXY_TARGET_RULE_"))
                && value.contains("=>")
        })
        .map(|(_, value)| value)
        .collect();
    let proxy_config: ProxyConfig = ProxyConfig::try_init(
        crate::config::var("PROXY_TARGET")?,
        &session_setup.cookie_name,
        proxy_rules,
    )?;
    let remaining_secs_threshold = crate::config::var("SESSION_REFRESH_THRESHOLD")?
        .parse::<_>()
        .context("Cannot parse SESSION_REFRESH_THRESHOLD")?;
    let login_redirects = crate::config::var_list("LOGIN_REDIRECT_APP_URIS")?;
    let oidc_redirects = crate::config::var_list("OIDC_REDIRECT_URIS")?;
    let logout_oidc_redirects = crate::config::var_list("LOGOUT_OIDC_REDIRECT_URIS")?;
    let logout_app_uris = crate::config::var_list("LOGOUT_REDIRECT_APP_URIS")?;
    let app_config = AppConfigurationState {
        login_app_settings: LoginAppSettings::new(login_redirects, oidc_redirects),
        logout_app_settings: LogoutAppSettings {
            client_id,
            logout_uri: crate::config::var("LOGOUT_SSO_URI")?,
            allowed_app_uris: UriAllowlist::new(logout_app_uris),
            allowed_oidc_redirect_uris: UriAllowlist::new(logout_oidc_redirects),
        },
    };

    let return_uris = config::registration_return_uris()?;
    let profile_settings = RegistrationAppSettings::new(return_uris.clone());
    let keycloak_admin = KeycloakAdmin::from_env().ok();

    let registration = if config::registration_enabled() {
        let human_check = sigma_human_check::HumanCheck::from_env()?;
        keycloak_admin.as_ref().map(|admin| RegistrationDeps {
            settings: profile_settings.clone(),
            admin: admin.clone(),
            adapter: RegistrationAdapter::from_env(),
            human_check: human_check.clone(),
        })
    } else {
        None
    };

    let profile = keycloak_admin.as_ref().map(|admin| ProfileDeps {
        settings: profile_settings.clone(),
        admin: admin.clone(),
    });

    let admin = keycloak_admin.map(|admin| AdminDeps { admin });

    let app = app(
        oidc_client,
        &session_layer,
        &proxy_config,
        pool,
        http::AppBuildOptions {
            remaining_secs_threshold,
            app_config,
            registration,
            profile,
            admin,
        },
    );

    let listener = http::port_listener().await?;
    let bind_addr = listener
        .local_addr()
        .context("Retrieve listening address")?;
    tracing::info!("Listening on http://{bind_addr}");

    axum::serve(listener, app?.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("error running server")?;
    Ok(())
}
