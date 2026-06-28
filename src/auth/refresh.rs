use anyhow::Result;
use axum::{
    Extension, Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_macros::debug_handler;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_sessions::Session;
use tracing::debug;

use crate::{
    auth::SessionTokens,
    session::{SESSION_KEY_JWT, SESSION_KEY_USERID},
};

use super::OIDCClient;

#[derive(Debug, Clone)]
pub(crate) struct RefreshLockManager {
    refreshing: Arc<Mutex<HashSet<String>>>,
    remaining_secs_threshold: u64,
}

impl RefreshLockManager {
    pub(crate) fn new(remaining_secs_threshold: u64) -> Self {
        Self {
            refreshing: Arc::new(Mutex::new(HashSet::new())),
            remaining_secs_threshold,
        }
    }

    async fn try_acquire(&self, userid: &str) -> Result<(), Response> {
        let mut users = self.refreshing.lock().await;
        if !users.insert(userid.to_string()) {
            return Err((StatusCode::CONFLICT, "Refresh pending...").into_response());
        }
        Ok(())
    }

    async fn release(&self, userid: &str) {
        self.refreshing.lock().await.remove(userid);
    }
}

#[debug_handler]
pub(crate) async fn refresh(
    Extension(refresh_lock): Extension<RefreshLockManager>,
    Extension(client): Extension<OIDCClient>,
    session: Session,
) -> Result<Response, Response> {
    let userid = session
        .get::<String>(SESSION_KEY_USERID)
        .await
        .unwrap_or(None)
        .ok_or_else(|| {
            debug!("No user id in session");
            (StatusCode::UNAUTHORIZED, "Unauthorized").into_response()
        })?;

    refresh_lock.try_acquire(&userid).await?;

    let Some(session_tokens) = session
        .get::<SessionTokens>(SESSION_KEY_JWT)
        .await
        .unwrap_or(None)
    else {
        refresh_lock.release(&userid).await;
        debug!("No tokens in session");
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized").into_response());
    };

    if session_tokens.ttl_gt(refresh_lock.remaining_secs_threshold)
        && !crate::config::conformance_mode()
    {
        refresh_lock.release(&userid).await;
        return Err((StatusCode::BAD_REQUEST, "Refresh too early").into_response());
    }

    let Some(refresh_token) = session_tokens.refresh_token() else {
        refresh_lock.release(&userid).await;
        debug!("No refresh token in session");
        return Err((StatusCode::BAD_REQUEST, "Refresh token missing").into_response());
    };

    let response = match client.refresh_token(refresh_token.as_str()).await {
        Ok(jwt) => {
            let _ = session.insert(SESSION_KEY_JWT, jwt).await;
            (StatusCode::OK, Json("Refresh successful")).into_response()
        }
        Err(e) => {
            debug!("Failed to refresh token: {e}");
            (
                StatusCode::BAD_REQUEST,
                "Failed to refresh token".to_string(),
            )
                .into_response()
        }
    };

    refresh_lock.release(&userid).await;
    Ok(response)
}

#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request, http::StatusCode, http::header::COOKIE};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use crate::auth::tests::MockSetup;

    #[tokio::test]
    async fn test_refresh() {
        let m = MockSetup::new().await;
        let mut app = m.router();
        let session_cookie = m.setup_authenticated_state(&mut app).await;
        m.setup_refresh_token_response("refresh nonce").await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/auth/refresh")
                    .header(COOKIE, session_cookie)
                    .method("POST")
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

        assert_eq!(status, StatusCode::OK, "response should be ok, but {body}");
    }
}
