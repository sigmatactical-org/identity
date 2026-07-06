use axum::{
    Extension, Router,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use sqlx::PgPool;
use tracing::error;

async fn health_check(Extension(pool): Extension<PgPool>) -> Response {
    let report = sigma_pg::health::build_report("identity", Some(&pool)).await;
    let status = StatusCode::from_u16(sigma_pg::health::http_status_code(&report))
        .unwrap_or(StatusCode::SERVICE_UNAVAILABLE);
    if status != StatusCode::OK {
        error!("identity health unhealthy: {:?}", report.checks);
    }
    (
        status,
        [("content-type", "application/json")],
        serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string()),
    )
        .into_response()
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
        let url = crate::config::test_database_url();
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

        assert_eq!(response.status(), StatusCode::OK);
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
        let report: sigma_pg::health::HealthReport = serde_json::from_str(&body).unwrap();
        assert_eq!(report.service, "identity");
        assert_eq!(report.status, sigma_pg::health::ServiceStatus::Healthy);
    }
}
