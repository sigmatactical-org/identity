use std::env;

use anyhow::Context;
use session::{SameSiteSetting, SessionSetup};
use tokio::signal;
use tracing::{debug, warn};

use crate::{
    auth::{
        AppConfigurationState, LoginAppSettings, LogoutAppSettings, LogoutBehavior, OIDCClient,
        allowlist::UriAllowlist,
    },
    config::validate_session_secret,
    http::{ProxyConfig, app},
    session::redis_cons,
};

mod auth;
mod config;
mod http;
mod monitoring;
mod session;

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

fn oidc_client_from_env() -> anyhow::Result<String> {
    crate::config::var("OIDC_CLIENT_ID")
}

async fn init_oidc_client(client_id: &str) -> anyhow::Result<OIDCClient> {
    let issuer_url = crate::config::var("OIDC_ISSUER_URL")?;
    let client_secret = crate::config::var("OIDC_CLIENT_SECRET")?;
    let auth_url = crate::config::var_optional("OIDC_AUTH_URL");
    OIDCClient::build(&issuer_url, client_id, &client_secret, auth_url).await
}

fn init_session_vars() -> anyhow::Result<SessionSetup> {
    let secure_disabled = crate::config::var_optional("SESSION_SECURE_COOKIE_DISABLED")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"));
    if secure_disabled {
        warn!("Disabling secure cookies is not recommended in production.");
    }

    let same_site =
        SameSiteSetting::from_env_string(crate::config::var_optional("SESSION_COOKIE_SAMESITE"));

    let secret = crate::config::var("SESSION_SECRET")?;
    validate_session_secret(&secret)?;

    Ok(SessionSetup {
        cookie_name: crate::config::var_optional("SESSION_COOKIE_NAME")
            .unwrap_or_else(|| "identity.sid".to_string()),
        cookie_path: crate::config::var_optional("SESSION_COOKIE_PATH")
            .unwrap_or_else(|| "/".to_string()),
        secret,
        ttl: None,
        secure_cookie: !secure_disabled,
        same_site,
    })
}

pub async fn run() -> anyhow::Result<()> {
    let connection_url = crate::config::var("REDIS_URL")?;
    let (store, client) = redis_cons(&connection_url).await?;
    let session_setup = init_session_vars()?;
    let session_layer = session_setup.get_session_layer(store)?;
    let client_id = oidc_client_from_env()?;
    let oidc_client = init_oidc_client(&client_id).await?;
    if config::conformance_mode() {
        debug!("OIDC conformance mode enabled (see /conformance/ harness)");
    }
    let proxy_rules: Vec<_> = env::vars()
        .filter_map(|(key, value)| {
            if (key.starts_with("IDENTITY_PROXY_TARGET_RULE_")
                || key.starts_with("RIDSER_PROXY_TARGET_RULE_"))
                && value.contains("=>")
            {
                Some(value)
            } else {
                None
            }
        })
        .collect();
    let proxy_config: ProxyConfig = ProxyConfig::try_init(
        crate::config::var("PROXY_TARGET")?,
        &session_setup.cookie_name,
        proxy_rules,
    )?;
    let remaining_secs_threshold = crate::config::var("SESSION_REFRESH_THRESHOLD")?
        .parse::<_>()
        .context("Cannot parse SESSION_REFRESH_THRESHOLD")?;
    let login_redirects = crate::config::var("LOGIN_REDIRECT_APP_URIS")?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let oidc_redirects = crate::config::var("OIDC_REDIRECT_URIS")?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let logout_oidc_redirects = crate::config::var("LOGOUT_OIDC_REDIRECT_URIS")?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let logout_app_uris = crate::config::var("LOGOUT_REDIRECT_APP_URIS")?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();
    let app_config = AppConfigurationState {
        login_app_settings: LoginAppSettings::new(login_redirects, oidc_redirects),
        logout_app_settings: LogoutAppSettings {
            client_id,
            logout_uri: crate::config::var("LOGOUT_SSO_URI")?,
            _behavior: LogoutBehavior::FrontChannelLogoutWithIdToken,
            allowed_app_uris: UriAllowlist::new(logout_app_uris),
            allowed_oidc_redirect_uris: UriAllowlist::new(logout_oidc_redirects),
        },
    };

    let app = app(
        oidc_client,
        &session_layer,
        &proxy_config,
        client,
        remaining_secs_threshold,
        app_config,
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
