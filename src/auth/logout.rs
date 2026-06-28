use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_macros::debug_handler;
use serde::Deserialize;
use tower_sessions::Session;
use tracing::{debug, trace, warn};
use url::Url;

use crate::session::{SESSION_KEY_JWT, SESSION_KEY_LOGOUT_APP_URI};

use super::{SessionTokens, allowlist::UriAllowlist};

#[derive(Clone, Debug)]
pub enum LogoutBehavior {
    FrontChannelLogoutWithIdToken,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LogoutQueryParams {
    #[serde(rename = "app_uri")]
    app_uri: String,
    #[serde(rename = "redirect_uri")]
    redirect_uri: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LogoutCallbackQueryParams {
    #[serde(rename = "app_uri")]
    app_uri: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LogoutAppSettings {
    pub(crate) client_id: String,
    pub(crate) logout_uri: String,
    pub(crate) _behavior: LogoutBehavior,
    pub(crate) allowed_app_uris: UriAllowlist,
    pub(crate) allowed_oidc_redirect_uris: UriAllowlist,
}

impl LogoutAppSettings {
    pub(crate) fn is_app_uri_allowed(&self, app_uri: &str) -> bool {
        trace!("logout app_uri check: {app_uri}");
        self.allowed_app_uris.is_allowed(app_uri)
    }

    pub(crate) fn is_oidc_redirect_allowed(&self, redirect_uri: &str) -> bool {
        trace!("logout oidc redirect check: {redirect_uri}");
        self.allowed_oidc_redirect_uris.is_allowed(redirect_uri)
    }
}

/// Keycloak returns the exact `post_logout_redirect_uri`; embed `app_uri` so the
/// callback does not depend on Redis session surviving the IdP round-trip.
fn build_post_logout_redirect_uri(
    redirect_uri: &str,
    app_uri: &str,
) -> Result<String, Box<Response>> {
    let mut url = Url::parse(redirect_uri)
        .map_err(|_| Box::new((StatusCode::BAD_REQUEST, "Invalid redirect_uri").into_response()))?;
    url.query_pairs_mut().append_pair("app_uri", app_uri);
    Ok(url.to_string())
}

fn build_logout_url(
    logout_uri: &str,
    client_id: &str,
    post_logout_redirect_uri: &str,
    id_token: &str,
) -> Result<String, Box<Response>> {
    let mut url = Url::parse(logout_uri).map_err(|_| {
        Box::new(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Invalid logout endpoint configuration",
            )
                .into_response(),
        )
    })?;
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("post_logout_redirect_uri", post_logout_redirect_uri);
        pairs.append_pair("client_id", client_id);
        if !id_token.is_empty() {
            pairs.append_pair("id_token_hint", id_token);
        }
    }
    Ok(url.to_string())
}

#[debug_handler]
pub(crate) async fn logout(
    State(logout_app_settings): State<LogoutAppSettings>,
    session: Session,
    logout_query_params: Query<LogoutQueryParams>,
) -> Response {
    if !logout_app_settings.is_app_uri_allowed(&logout_query_params.app_uri) {
        debug!(
            "logout app_uri not allowed: {}",
            logout_query_params.app_uri
        );
        return (StatusCode::BAD_REQUEST, "Invalid app_uri").into_response();
    }
    if !logout_app_settings.is_oidc_redirect_allowed(&logout_query_params.redirect_uri) {
        debug!(
            "logout redirect_uri not allowed: {}",
            logout_query_params.redirect_uri
        );
        return (StatusCode::BAD_REQUEST, "Invalid redirect_uri").into_response();
    }

    let _ = session
        .insert(
            SESSION_KEY_LOGOUT_APP_URI,
            logout_query_params.app_uri.clone(),
        )
        .await;
    let _ = session.save().await;

    let post_logout_redirect_uri = match build_post_logout_redirect_uri(
        &logout_query_params.redirect_uri,
        &logout_query_params.app_uri,
    ) {
        Ok(uri) => uri,
        Err(resp) => return *resp,
    };

    let session_tokens: Option<SessionTokens> = session.get(SESSION_KEY_JWT).await.unwrap_or(None);
    let id_token = session_tokens.map(|st| st.id_token).unwrap_or_default();

    match build_logout_url(
        &logout_app_settings.logout_uri,
        &logout_app_settings.client_id,
        &post_logout_redirect_uri,
        &id_token,
    ) {
        Ok(uri) => Redirect::to(uri.as_str()).into_response(),
        Err(resp) => *resp,
    }
}

#[debug_handler]
pub(crate) async fn logout_callback(
    State(logout_app_settings): State<LogoutAppSettings>,
    session: Session,
    callback_query_params: Query<LogoutCallbackQueryParams>,
) -> Response {
    let app_uri = callback_query_params
        .app_uri
        .clone()
        .or(session
            .get::<String>(SESSION_KEY_LOGOUT_APP_URI)
            .await
            .unwrap_or_default())
        .unwrap_or_else(|| {
            warn!("logout app_uri not found in callback query or session");
            "/".to_string()
        });

    let _: Result<(), _> = session.flush().await;
    if !logout_app_settings.is_app_uri_allowed(&app_uri) {
        return (StatusCode::BAD_REQUEST, "Invalid app_uri").into_response();
    }
    Redirect::to(&app_uri).into_response()
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{
            Request, StatusCode,
            header::{COOKIE, LOCATION, SET_COOKIE},
        },
    };
    use http_body_util::BodyExt;
    use tower::{Service, ServiceExt};

    use crate::auth::tests::MockSetup;

    #[tokio::test]
    async fn test_handles_anonymous_state_with_valid_app_uris() {
        let m = MockSetup::new().await;
        let mut app = m.router();
        let urilist = vec![
            "http://logout.example.com".to_string(),
            "http://example.org/it/index".to_string(),
        ];

        for app_uri in urilist {
            let response = ServiceExt::<Request<Body>>::ready(&mut app)
                .await
                .unwrap()
                .call(
                    Request::builder()
                        .uri(format!(
                            "/auth/logout?app_uri={app_uri}&redirect_uri=http://example.com/auth/logoutcallback"
                        ))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = response.status();
            let body = String::from_utf8(
                response
                    .into_body()
                    .collect()
                    .await
                    .expect("collect")
                    .to_bytes()
                    .to_vec(),
            )
            .unwrap();

            assert_eq!(
                status,
                StatusCode::SEE_OTHER,
                "response should be redirect, but {body}"
            );
        }
    }

    #[tokio::test]
    async fn test_handles_anonymous_state_with_invalid_app_uris() {
        let m = MockSetup::new().await;
        let mut app = m.router();
        let urilist = vec![
            "/".to_string(),
            "http://start.com/".to_string(),
            "http://example.org/it/index2".to_string(),
            "http://example.org/it/index/".to_string(),
            "http://logout.example.com/".to_string(),
            "http://logout.example.com:8000".to_string(),
            "https://logout.example.com".to_string(),
        ];

        for app_uri in urilist {
            let response = ServiceExt::<Request<Body>>::ready(&mut app)
                .await
                .unwrap()
                .call(
                    Request::builder()
                        .uri(format!(
                            "/auth/logout?app_uri={app_uri}&redirect_uri=http://example.com/auth/logoutcallback"
                        ))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "app_uri {app_uri} should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn test_handles_authenticated_state() {
        let m = MockSetup::new().await;
        let mut app = m.router();
        let session_cookie = m.setup_authenticated_state(&mut app).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/logout?app_uri=http://logout.example.com&redirect_uri=http://example.com/auth/logoutcallback")
                    .header(COOKIE, session_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = response
            .headers()
            .get(LOCATION)
            .map(|hv| hv.to_str().unwrap().to_string())
            .unwrap_or_default();
        assert!(
            location.contains("client_id="),
            "logout URL should include client_id"
        );
        assert!(
            location.contains("id_token_hint="),
            "logout URL should include id_token_hint"
        );
        assert!(
            location.contains("post_logout_redirect_uri="),
            "logout URL should include post_logout_redirect_uri"
        );
        assert!(
            location.contains("app_uri"),
            "logout URL should embed app_uri in post_logout_redirect_uri"
        );
    }

    #[tokio::test]
    async fn test_logout_callback_reads_app_uri_from_query() {
        let m = MockSetup::new().await;
        let mut app = m.router();
        let app_uri = "http://logout.example.com".to_string();

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(
                Request::builder()
                    .uri(format!("/auth/logoutcallback?app_uri={app_uri}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let location = response
            .headers()
            .get(LOCATION)
            .map(|hv| hv.to_str().unwrap().to_string());
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(Some(app_uri), location);
    }

    #[tokio::test]
    async fn test_handles_app_redirect() {
        let m = MockSetup::new().await;
        let mut app = m.router();
        let session_cookie = m.setup_authenticated_state(&mut app).await;
        let app_uri = "http://logout.example.com".to_string();

        let _response_logout1 = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(
                Request::builder()
                    .uri(format!(
                        "/auth/logout?app_uri={app_uri}&redirect_uri=http://example.com/auth/logoutcallback"
                    ))
                    .header(COOKIE, session_cookie.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(
                Request::builder()
                    .uri("/auth/logoutcallback".to_string())
                    .header(COOKIE, session_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let location = response
            .headers()
            .get(LOCATION)
            .map(|hv| hv.to_str().unwrap().to_string());
        let cookie = response
            .headers()
            .get(SET_COOKIE)
            .map(|hv| hv.to_str().unwrap().to_string())
            .unwrap_or_default();

        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        assert_eq!(Some(app_uri.to_string()), location);
        assert!(
            cookie.contains("; Max-Age=0;"),
            "Cookie should be marked expired, but {cookie}"
        );
    }
}
