//! Conformance-suite harness endpoints (enabled with IDENTITY_CONFORMANCE_MODE).

use axum::{
    Extension, Json,
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_macros::debug_handler;
use serde::Serialize;
use tower_sessions::Session;

use crate::auth::{LoginCallbackSessionParameters, OIDCClient};
use crate::session::SESSION_KEY_LOGIN_CALLBACK;

use super::callback::{CallbackQueryParams, TokenExchangeData, callback_post_token_exchange};

#[derive(Serialize)]
struct DiscoverResponse {
    issuer: String,
    status: &'static str,
}

#[debug_handler]
pub(crate) async fn discover(Extension(oidc): Extension<OIDCClient>) -> impl IntoResponse {
    match oidc.refresh_provider_metadata().await {
        Ok(issuer) => (
            StatusCode::OK,
            Json(DiscoverResponse {
                issuer,
                status: "ok",
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
        )
            .into_response(),
    }
}

/// POST /auth/callback for response_mode=form_post (OAuth form body).
#[debug_handler]
pub(crate) async fn callback_form_post(
    Extension(oidc_client): Extension<OIDCClient>,
    session: Session,
    axum::Form(params): axum::Form<CallbackQueryParams>,
) -> Result<Response, Response> {
    let login_callback_session_params = session
        .get::<LoginCallbackSessionParameters>(SESSION_KEY_LOGIN_CALLBACK)
        .await
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Invalid session").into_response())?
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid session").into_response())?;
    let _: Result<Option<()>, _> = session.remove(SESSION_KEY_LOGIN_CALLBACK).await;

    if params.state != login_callback_session_params.csrf_token {
        return Err((StatusCode::BAD_REQUEST, "Invalid request").into_response());
    }

    if let Some(error) = &params.error {
        return Ok(Redirect::to(&format!(
            "{}?error={}",
            login_callback_session_params.app_uri, error
        ))
        .into_response());
    }

    let (jwt, userid) = oidc_client
        .exchange_code(TokenExchangeData {
            code: params.code.clone().unwrap_or_default(),
            nonce: login_callback_session_params.nonce,
            pkce_verifier: login_callback_session_params.pkce_verifier,
            redirect_uri: login_callback_session_params.redirect_uri.clone(),
        })
        .await
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Login failure").into_response())?;

    callback_post_token_exchange(&session, jwt, userid).await;

    Ok(Redirect::to(&login_callback_session_params.app_uri).into_response())
}
