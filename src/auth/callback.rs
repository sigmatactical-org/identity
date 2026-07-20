mod callback_query_params;
mod token_exchange_data;
pub(crate) use callback_query_params::CallbackQueryParams;
pub(crate) use token_exchange_data::TokenExchangeData;

use axum::{
    Extension,
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_macros::debug_handler;
use oauth2::reqwest::Url;
use tower_sessions::Session;
use tracing::{error, info};

use crate::{
    auth::{LoginCallbackSessionParameters, OIDCClient},
    session::{
        SESSION_KEY_CSRF_TOKEN, SESSION_KEY_JWT, SESSION_KEY_LOGIN_CALLBACK, SESSION_KEY_USERID,
        purge_store_and_regenerate_session,
    },
};

use super::{SessionTokens, random_alphanumeric_string};

pub(crate) async fn callback_post_token_exchange(
    session: &Session,
    jwt: SessionTokens,
    userid: String,
) {
    purge_store_and_regenerate_session(session).await;

    let _ = session.insert(SESSION_KEY_JWT, jwt).await;
    let _ = session
        .insert(SESSION_KEY_CSRF_TOKEN, random_alphanumeric_string(24))
        .await;
    let _ = session.insert(SESSION_KEY_USERID, userid).await;
}

/// The shared post-authorization sequence: validate state against the stored
/// login parameters, surface an IdP error back to the app, otherwise exchange
/// the code and establish the session. Used by both the GET callback and the
/// `response_mode=form_post` POST callback.
pub(crate) async fn handle_callback(
    oidc_client: &OIDCClient,
    session: &Session,
    params: CallbackQueryParams,
) -> Result<Response, Response> {
    let login_callback_session_params = session
        .get::<LoginCallbackSessionParameters>(SESSION_KEY_LOGIN_CALLBACK)
        .await
        .map_err(|session_err| {
            error!("Reading session error in callback: {:?}", session_err);
            (StatusCode::INTERNAL_SERVER_ERROR, "Invalid session").into_response()
        })?
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid session").into_response())?;
    let _: Result<Option<()>, _> = session.remove(SESSION_KEY_LOGIN_CALLBACK).await;

    if params.state != login_callback_session_params.csrf_token {
        return Err((StatusCode::BAD_REQUEST, "Invalid request").into_response());
    }

    // If the callback carries an error, redirect to the app with it appended.
    if let Some(error) = &params.error {
        let app_uri = Url::parse(&login_callback_session_params.app_uri)
            .map(|mut url| {
                url.query_pairs_mut().append_pair("error", error.as_str());
                url.to_string()
            })
            .map_err(|e| {
                error!(
                    "Failed to parse redirect URI: {} {:?}",
                    &login_callback_session_params.app_uri, e
                );
                (StatusCode::BAD_REQUEST, "Invalid app_uri").into_response()
            })?;
        return Ok(Redirect::to(&app_uri).into_response());
    }

    let (jwt, userid) = oidc_client
        .exchange_code(TokenExchangeData {
            code: params.code.unwrap_or_default(),
            nonce: login_callback_session_params.nonce,
            pkce_verifier: login_callback_session_params.pkce_verifier,
            redirect_uri: login_callback_session_params.redirect_uri.clone(),
        })
        .await
        .map_err(|e| {
            info!("Failed to exchange code: {:?}", e);
            (StatusCode::UNAUTHORIZED, "Login failure").into_response()
        })?;

    callback_post_token_exchange(session, jwt, userid).await;

    Ok(Redirect::to(&login_callback_session_params.app_uri).into_response())
}

#[debug_handler]
pub(crate) async fn callback(
    Extension(oidc_client): Extension<OIDCClient>,
    session: Session,
    Query(params): Query<CallbackQueryParams>,
) -> Result<Response, Response> {
    handle_callback(&oidc_client, &session, params).await
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use axum::{
        body::Body,
        http::{
            Request,
            header::{COOKIE, LOCATION, SET_COOKIE},
        },
    };
    use http_body_util::BodyExt; // for `collect`
    use hyper::Uri;
    use tower::{Service, ServiceExt};

    use crate::auth::tests::MockSetup;

    use super::*;

    #[tokio::test]
    async fn test_callback_provides_session() {
        // Arrange
        let m = MockSetup::new().await;
        let mut app = m.router();
        let code: String = random_alphanumeric_string(20);

        // Act
        let request = Request::builder().uri("/auth/login?app_uri=http://example.com&redirect_uri=http://example.com/auth/callback&scope=openid".to_string()).body(Body::empty()).unwrap();
        let response1 = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();
        let cookie1 = response1.headers().get(SET_COOKIE).unwrap();
        let redirect_uri1 = response1.headers().get(LOCATION).unwrap();
        let uri = Uri::from_str(redirect_uri1.to_str().unwrap()).unwrap();
        let state: String = uri
            .query()
            .unwrap_or_default()
            .split('&')
            .find(|s| s.starts_with("state="))
            .expect("Redirect uri should have state")
            .split('=')
            .skip(1)
            .take(1)
            .collect();

        m.setup_id_token_nonce(redirect_uri1).await;

        let request = Request::builder()
            .uri(format!("/auth/callback?code={code}&state={state}"))
            .header(COOKIE, cookie1.clone())
            .body(Body::empty())
            .unwrap();
        let response2 = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(request)
            .await
            .unwrap();
        let status2 = response2.status();
        let headers2 = response2.headers().clone();
        let body = String::from_utf8(
            response2
                .into_body()
                .collect()
                .await
                .expect("collect")
                .to_bytes()
                .to_vec(),
        )
        .unwrap();

        // Assert
        assert_eq!(
            status2,
            StatusCode::SEE_OTHER,
            "response2 should be redirect, but {body}"
        );
        let cookie2 = headers2.get(SET_COOKIE).unwrap();
        assert_ne!(
            cookie1.to_str().unwrap(),
            cookie2.to_str().unwrap(),
            "Second cookie should be different"
        );
    }
}
