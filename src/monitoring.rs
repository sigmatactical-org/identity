use axum::{
    Extension, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use tracing::error;

async fn health_check(Extension(pool): Extension<PgPool>) -> Response {
    if let Err(err) = sigma_pg::ping(&pool).await {
        error!("Failed to connect to PostgreSQL: {err:?}");
        return (StatusCode::SERVICE_UNAVAILABLE, "Unhealthy").into_response();
    }

    (StatusCode::OK, "OK").into_response()
}

pub(crate) fn health_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/up", get(|| async { "up" }))
        .route("/health", get(health_check).layer(Extension(pool)))
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    async fn test_pool() -> PgPool {
        let url = crate::config::var_optional("TEST_DATABASE_URL")
            .unwrap_or_else(|| sigma_pg::DEFAULT_DATABASE_URL.to_string());
        sigma_pg::connect_url(&url)
            .await
            .expect("PostgreSQL required for tests")
    }

    #[tokio::test]
    async fn test_up() {
        let pool = test_pool().await;
        let app = health_routes(pool);

        let response = app
            .oneshot(Request::builder().uri("/up").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("collect")
            .to_bytes()
            .to_vec();
        assert_eq!(&body[..], b"up");
    }

    #[tokio::test]
    async fn test_health() {
        let pool = test_pool().await;
        let app = health_routes(pool);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
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

        assert_eq!(status, StatusCode::OK, "Expected 200 OK, but {body}");
        assert_eq!(body, "OK");
    }
}
