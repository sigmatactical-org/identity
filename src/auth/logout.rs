mod logout_app_settings;
mod logout_behavior;
mod logout_callback_query_params;
mod logout_query_params;
pub use logout_app_settings::LogoutAppSettings;
pub use logout_behavior::LogoutBehavior;
pub(crate) use logout_callback_query_params::LogoutCallbackQueryParams;
pub(crate) use logout_query_params::LogoutQueryParams;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite as CookieSameSite},
};
use axum_macros::debug_handler;
use time::Duration;
use tower_sessions::Session;
use tracing::{debug, warn};
use url::Url;

use crate::session::{
    LOGOUT_APP_URI_COOKIE, SESSION_KEY_JWT, SESSION_KEY_LOGOUT_APP_URI, SameSiteSetting,
};

use super::SessionTokens;

fn session_cookie_secure() -> bool {
    !crate::config::var_optional("SESSION_SECURE_COOKIE_DISABLED")
        .is_some_and(|v| v.eq_ignore_ascii_case("true"))
}

fn session_cookie_same_site() -> SameSiteSetting {
    SameSiteSetting::from_env_string(crate::config::var_optional("SESSION_COOKIE_SAMESITE"))
}

fn logout_app_uri_cookie(app_uri: &str) -> Cookie<'static> {
    let mut builder = Cookie::build((LOGOUT_APP_URI_COOKIE, app_uri.to_string()))
        .http_only(true)
        .path("/auth")
        .max_age(Duration::minutes(10));
    if session_cookie_secure() {
        builder = builder.secure(true);
    }
    builder = builder.same_site(match session_cookie_same_site() {
        SameSiteSetting::None => CookieSameSite::None,
        SameSiteSetting::Lax => CookieSameSite::Lax,
        SameSiteSetting::Strict => CookieSameSite::Strict,
    });
    builder.build()
}

fn clear_logout_app_uri_cookie() -> Cookie<'static> {
    Cookie::build((LOGOUT_APP_URI_COOKIE, ""))
        .http_only(true)
        .path("/auth")
        .max_age(Duration::seconds(0))
        .build()
}

/// Build the IdP logout URL with client_id and optional id_token_hint.
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
    jar: CookieJar,
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

    let jar = jar.add(logout_app_uri_cookie(&logout_query_params.app_uri));

    let post_logout_redirect_uri = logout_query_params.redirect_uri.clone();

    let id_token = if crate::config::logout_send_id_token_hint() {
        session
            .get::<SessionTokens>(SESSION_KEY_JWT)
            .await
            .unwrap_or(None)
            .map(|st| st.id_token)
            .unwrap_or_default()
    } else {
        String::new()
    };

    match build_logout_url(
        &logout_app_settings.logout_uri,
        &logout_app_settings.client_id,
        &post_logout_redirect_uri,
        &id_token,
    ) {
        Ok(uri) => (jar, Redirect::to(uri.as_str())).into_response(),
        Err(resp) => *resp,
    }
}

#[debug_handler]
pub(crate) async fn logout_callback(
    State(logout_app_settings): State<LogoutAppSettings>,
    session: Session,
    jar: CookieJar,
    callback_query_params: Query<LogoutCallbackQueryParams>,
) -> Response {
    let app_uri = callback_query_params
        .app_uri
        .clone()
        .or_else(|| {
            jar.get(LOGOUT_APP_URI_COOKIE)
                .map(|cookie| cookie.value().to_string())
        })
        .or(session
            .get::<String>(SESSION_KEY_LOGOUT_APP_URI)
            .await
            .unwrap_or_default())
        .unwrap_or_else(|| {
            warn!("logout app_uri not found in callback query, cookie, or session");
            "/".to_string()
        });

    let _: Result<(), _> = session.flush().await;
    if !logout_app_settings.is_app_uri_allowed(&app_uri) {
        return (StatusCode::BAD_REQUEST, "Invalid app_uri").into_response();
    }

    let jar = jar.remove(clear_logout_app_uri_cookie());
    (jar, Redirect::to(&app_uri)).into_response()
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
    use crate::session::LOGOUT_APP_URI_COOKIE;

    use super::logout_app_uri_cookie;

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
        let set_cookie = response
            .headers()
            .get(SET_COOKIE)
            .map(|hv| hv.to_str().unwrap().to_string())
            .unwrap_or_default();
        assert!(
            location.contains("client_id="),
            "logout URL should include client_id"
        );
        assert!(
            location.contains("post_logout_redirect_uri="),
            "logout URL should include post_logout_redirect_uri"
        );
        assert!(
            !location.contains("id_token_hint="),
            "logout URL should not include id_token_hint by default"
        );
        assert!(
            set_cookie.contains(LOGOUT_APP_URI_COOKIE),
            "logout should set app_uri cookie, got {set_cookie}"
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
    async fn test_logout_callback_reads_app_uri_from_cookie() {
        let m = MockSetup::new().await;
        let mut app = m.router();
        let app_uri = "http://logout.example.com".to_string();
        let logout_cookie = logout_app_uri_cookie(&app_uri);

        let response = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(
                Request::builder()
                    .uri("/auth/logoutcallback")
                    .header(COOKIE, logout_cookie.to_string())
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
