use anyhow::Result;
use axum::http::HeaderValue;
use axum::{
    Extension,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_macros::debug_handler;
use serde::Deserialize;
use tower_sessions::Session;
use tracing::{debug, error, trace};

use crate::{
    auth::{
        LoginCallbackSessionParameters,
        allowlist::{UriAllowlist, browser_origins_from_entries},
        oidcclient::AuthorizeRequestData,
        random_alphanumeric_string,
    },
    session::{SESSION_KEY_LOGIN_CALLBACK, purge_store_and_regenerate_session},
};

use super::OIDCClient;

#[derive(Debug, Deserialize)]
pub(crate) struct LoginQueryParams {
    #[serde(rename = "app_uri")]
    app_uri: String,
    #[serde(rename = "redirect_uri")]
    redirect_uri: String,
    #[serde(rename = "scope")]
    scope: String,
    #[serde(rename = "ui_locales")]
    ui_locales: Option<String>,
    #[serde(rename = "prompt")]
    prompt: Option<String>,
    #[serde(rename = "kc_idp_hint")]
    kc_idp_hint: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LoginAppSettings {
    app_uris: UriAllowlist,
    redirect_uris: UriAllowlist,
    cors_origins: Vec<HeaderValue>,
}

impl LoginAppSettings {
    pub fn new(app_uri_entries: Vec<String>, redirect_uri_entries: Vec<String>) -> Self {
        Self {
            cors_origins: browser_origins_from_entries(&app_uri_entries),
            app_uris: UriAllowlist::new(app_uri_entries),
            redirect_uris: UriAllowlist::new(redirect_uri_entries),
        }
    }

    pub(crate) fn cors_origins(&self) -> &[HeaderValue] {
        &self.cors_origins
    }

    pub(crate) fn is_app_uri_allowed(&self, app_uri: &str) -> bool {
        trace!("login app_uri check: {app_uri}");
        self.app_uris.is_allowed(app_uri)
    }

    pub(crate) fn is_redirect_uri_allowed(&self, redirect_uri: &str) -> bool {
        trace!("login redirect_uri check: {redirect_uri}");
        self.redirect_uris.is_allowed(redirect_uri)
    }
}

#[debug_handler]
pub(crate) async fn login(
    State(login_app_settings): State<LoginAppSettings>,
    Extension(oidc_client): Extension<OIDCClient>,
    session: Session,
    login_query_params: Query<LoginQueryParams>,
) -> Result<Response, Response> {
    if !login_app_settings.is_app_uri_allowed(login_query_params.app_uri.as_str()) {
        debug!("app_uri {} is not allowed", login_query_params.app_uri);
        return Err((StatusCode::BAD_REQUEST, "Invalid app_uri").into_response());
    }
    if !login_app_settings.is_redirect_uri_allowed(login_query_params.redirect_uri.as_str()) {
        debug!(
            "redirect_uri {} is not allowed",
            login_query_params.redirect_uri
        );
        return Err((StatusCode::BAD_REQUEST, "Invalid redirect_uri").into_response());
    }
    purge_store_and_regenerate_session(&session).await;
    let state: String = random_alphanumeric_string(20);
    let d = oidc_client
        .authorize_data(AuthorizeRequestData {
            redirect_uri: login_query_params.redirect_uri.clone(),
            scope: login_query_params.scope.clone(),
            state,
            ui_locales: login_query_params.ui_locales.clone(),
            prompt: login_query_params.prompt.clone(),
            kc_idp_hint: login_query_params.kc_idp_hint.clone(),
            register_return_url: Some(login_query_params.app_uri.clone()),
        })
        .await
        .map_err(|e| {
            error!("Failed to build authorization url {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Server failure").into_response()
        })?;
    let auth_url = d.auth_url.as_str();

    let _ = session
        .insert(
            SESSION_KEY_LOGIN_CALLBACK,
            LoginCallbackSessionParameters {
                app_uri: login_query_params.app_uri.clone(),
                nonce: d.nonce,
                csrf_token: d.csrf_token,
                pkce_verifier: d.pkce_verifier,
                redirect_uri: login_query_params.redirect_uri.clone(),
                scopes: login_query_params.scope.clone(),
            },
        )
        .await;

    debug!("login redirecting to {auth_url}");
    Ok(Redirect::to(auth_url).into_response())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{
            Request,
            header::{COOKIE, LOCATION, SET_COOKIE},
        },
    };
    use tower::{Service, ServiceExt};

    use crate::auth::tests::MockSetup;

    use super::*;

    const REDIRECT_URI: &str = "http://example.com/auth/callback";

    #[tokio::test]
    async fn test_login_sends_redirect() {
        let m = MockSetup::new().await;
        let app = m.router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/auth/login?app_uri=http://example.com&redirect_uri={REDIRECT_URI}&scope=openid&state=xyz"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
    }

    #[tokio::test]
    async fn test_login_sends_new_session_cookie() {
        let m = MockSetup::new().await;
        let mut app = m.router();
        let uri = format!(
            "/auth/login?app_uri=http://example.com&redirect_uri={REDIRECT_URI}&scope=openid&state=xyz"
        );

        let request = Request::builder().uri(&uri).body(Body::empty()).unwrap();
        let response1 = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();
        let cookie1 = response1.headers().get(SET_COOKIE).unwrap();

        let request = Request::builder()
            .uri(&uri)
            .header(COOKIE, cookie1.clone())
            .body(Body::empty())
            .unwrap();
        let response2 = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();
        let cookie2 = response2.headers().get(SET_COOKIE).unwrap();

        assert_eq!(response1.status(), StatusCode::SEE_OTHER);
        assert_eq!(response2.status(), StatusCode::SEE_OTHER);
        assert_ne!(cookie1.to_str().unwrap(), cookie2.to_str().unwrap());
    }

    #[tokio::test]
    async fn test_app_uri_invalid() {
        let m = MockSetup::new().await;
        let mut app = m.router();

        for app_uri in [
            "https://example.com",
            "http://example.com/",
            "http://example.org/my/app",
            "http://example.fr",
            "http://example.com:8080",
        ] {
            let response = ServiceExt::<Request<Body>>::ready(&mut app)
                .await
                .unwrap()
                .call(
                    Request::builder()
                        .uri(format!(
                            "/auth/login?app_uri={app_uri}&redirect_uri={REDIRECT_URI}&scope=openid&state=xyz"
                        ))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
    }

    #[tokio::test]
    async fn test_redirect_uri_invalid() {
        let m = MockSetup::new().await;
        let mut app = m.router();

        for redirect_uri in [
            "http://evil.com/auth/callback",
            "http://example.com/callback",
        ] {
            let response = ServiceExt::<Request<Body>>::ready(&mut app)
                .await
                .unwrap()
                .call(
                    Request::builder()
                        .uri(format!(
                            "/auth/login?app_uri=http://example.com&redirect_uri={redirect_uri}&scope=openid"
                        ))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
    }

    #[tokio::test]
    async fn test_app_uri_valid() {
        let m = MockSetup::new().await;
        let mut app = m.router();

        for app_uri in [
            "http://example.com",
            "http://example.org/my/app/*",
            "http://example.org/my/app/",
            "http://example.org/my/app/abc/",
            "http://example.org/my/app/kp/h?id=34",
        ] {
            let response = ServiceExt::<Request<Body>>::ready(&mut app)
                .await
                .unwrap()
                .call(
                    Request::builder()
                        .uri(format!(
                            "/auth/login?app_uri={app_uri}&redirect_uri={REDIRECT_URI}&scope=openid&state=xyz"
                        ))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::SEE_OTHER);
        }
    }

    #[tokio::test]
    async fn test_login_sends_redirect_with_ui_locales() {
        let m = MockSetup::new().await;
        let app = m.router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/auth/login?app_uri=http://example.com&redirect_uri={REDIRECT_URI}&scope=openid&state=xyz&ui_locales=de"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let url = response.headers().get(LOCATION).unwrap().to_str().unwrap();
        assert!(url.contains("ui_locales=de"));
    }

    #[tokio::test]
    async fn test_login_sends_redirect_with_prompt_none() {
        let m = MockSetup::new().await;
        let app = m.router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/auth/login?app_uri=http://example.com&redirect_uri={REDIRECT_URI}&scope=openid&state=xyz&prompt=none"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let url = response.headers().get(LOCATION).unwrap().to_str().unwrap();
        assert!(url.contains("prompt=none"));
    }

    #[tokio::test]
    async fn test_login_sends_redirect_with_kc_idp_hint() {
        let m = MockSetup::new().await;
        let app = m.router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/auth/login?app_uri=http://example.com&redirect_uri={REDIRECT_URI}&scope=openid&state=xyz&kc_idp_hint=some-idp"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let url = response.headers().get(LOCATION).unwrap().to_str().unwrap();
        assert!(url.contains("kc_idp_hint=some-idp"));
    }
}
