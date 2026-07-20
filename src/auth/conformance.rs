//! Conformance-suite harness endpoints (enabled with IDENTITY_CONFORMANCE_MODE).

mod discover_response;
pub(crate) use discover_response::DiscoverResponse;

use axum::{
    Extension, Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_macros::debug_handler;
use tower_sessions::Session;

use crate::auth::OIDCClient;

use super::callback::{CallbackQueryParams, handle_callback};

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
    handle_callback(&oidc_client, &session, params).await
}
