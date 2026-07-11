use std::{
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};
use axum::{
    Extension, Router,
    body::Body,
    http::{
        HeaderName, HeaderValue, Method, StatusCode, Uri,
        header::{AUTHORIZATION, CONTENT_TYPE, COOKIE, HOST},
    },
    response::{Html, IntoResponse, Response},
    routing::delete,
    routing::get,
};
use axum_extra::extract::CookieJar;
use axum_macros::debug_handler;
use hyper_rustls::HttpsConnector;
use sqlx::PgPool;
use tower::ServiceBuilder;
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};
use tower_sessions::{Session, SessionManagerLayer, SessionStore, service::PrivateCookie};
use tracing::{debug, error, warn};

use crate::{
    auth::{
        AdminDeps, AppConfigurationState, OIDCClient, ProfileDeps, RegistrationDeps, SessionTokens,
        auth_routes, register_routes, token_has_any_realm_role,
    },
    config,
    monitoring::health_routes,
    session::{SESSION_KEY_CSRF_TOKEN, SESSION_KEY_JWT},
    templates,
};

const THEMED_INDEX_APPS: &[&str] = &["exampleapp", "conformance"];

pub(crate) static HEADER_KEY_CSRF_TOKEN: &str = "x-csrf-token";
pub(crate) static HEADER_KEY_INTERNAL_TOKEN: &str = "x-sigma-internal-token";

pub(crate) type ProxyClient = hyper_util::client::legacy::Client<
    HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    Body,
>;

#[derive(Debug, Clone)]
pub(crate) struct ExtraProxyRoute {
    path: String,
    target: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ProxyConfig {
    base_url: String,
    cookie_name: String,
    extra_routes: Vec<ExtraProxyRoute>,
}

impl ProxyConfig {
    fn rewrite_uri(&self, uri: &str) -> String {
        self.extra_routes
            .iter()
            .find_map(|x| {
                if uri.starts_with(x.path.as_str()) {
                    let path = &uri[x.path.len()..];
                    Some(format!("{}{}", x.target.as_str(), path))
                } else {
                    None
                }
            })
            .unwrap_or(format!("{}{}", &self.base_url, uri))
    }

    pub(crate) fn try_init(
        base_url: String,
        cookie_name: &str,
        extra_routes: Vec<String>,
    ) -> Result<Self> {
        let uri = Uri::from_str(base_url.as_str())?;
        if uri.host().is_none() {
            return Err(anyhow!("Missing host"));
        }
        let mut extra_routes: Vec<ExtraProxyRoute> = extra_routes
            .into_iter()
            .filter_map(|s| {
                s.split_once("=>").map(|(path, target)| ExtraProxyRoute {
                    path: path.to_string(),
                    target: target.to_string(),
                })
            })
            .collect();
        extra_routes.sort_by_key(|b| std::cmp::Reverse(b.path.len()));
        Ok(Self {
            base_url,
            cookie_name: cookie_name.to_string(),
            extra_routes,
        })
    }
}

pub(crate) async fn port_listener() -> Result<tokio::net::TcpListener> {
    let port_str = config::listen_port();
    let port_parsed = port_str
        .parse::<u16>()
        .context("BIND_PORT/PORT must be a number between 1 and 65535")?;

    let bind_addr = config::var_optional("BIND_ADDRESS").unwrap_or_else(|| "::".to_string());
    let addr: SocketAddr = if bind_addr.contains(':') {
        format!("[{bind_addr}]:{port_parsed}")
            .parse()
            .context("Invalid IPv6 BIND_ADDRESS")?
    } else {
        SocketAddr::new(
            bind_addr
                .parse::<IpAddr>()
                .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)),
            port_parsed,
        )
    };

    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => Ok(listener),
        Err(_) => tokio::net::TcpListener::bind(SocketAddr::new(
            IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
            port_parsed,
        ))
        .await
        .context("Failed to bind to the specified address and port"),
    }
}

fn is_themed_index_app(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| THEMED_INDEX_APPS.contains(&name))
}

fn walk_dir(path: &str) -> Result<Vec<PathBuf>> {
    let root = Path::new(path);
    if !root.is_dir() {
        warn!("Static files directory `{path}` not found; serving API/auth only");
        return Ok(Vec::new());
    }
    let files = std::fs::read_dir(root).context("Reading static files directory")?;
    let mut paths = Vec::new();
    for entry in files {
        match entry {
            Ok(entry) => {
                if entry.file_type()?.is_dir() {
                    paths.append(&mut walk_dir(&entry.path().to_string_lossy())?);
                }
                if entry.file_name() == "index.html"
                    && let Some(parent) = entry.path().parent()
                    && !is_themed_index_app(parent)
                {
                    paths.push(parent.to_owned());
                }
            }
            Err(e) => warn!("File system error: {e:?}"),
        }
    }
    Ok(paths)
}

fn themed_page_routes() -> Router {
    let mut router = Router::new()
        .route("/exampleapp", get(exampleapp_page))
        .route("/exampleapp/", get(exampleapp_page));

    if config::conformance_mode() {
        router = router
            .route("/conformance", get(conformance_page))
            .route("/conformance/", get(conformance_page));
    }

    router
}

fn mount_themed_app_assets(mut app: Router, files_root: &Path) -> Router {
    for app_name in THEMED_INDEX_APPS {
        let app_dir = files_root.join(app_name);
        if !app_dir.is_dir() {
            continue;
        }
        debug!("Serving themed app assets from {app_dir:?}");
        let Ok(entries) = std::fs::read_dir(&app_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let route = format!("/{app_name}/{filename}");
            app = app.route_service(&route, ServeFile::new(path));
        }
    }
    app
}

async fn exampleapp_page() -> impl IntoResponse {
    match templates::render_exampleapp_html() {
        Ok(html) => Html(html).into_response(),
        Err(error) => {
            error!("Failed to render example app page: {error}");
            sigma_theme::axum::internal_server_error().await
        }
    }
}

async fn conformance_page() -> impl IntoResponse {
    match templates::render_conformance_html() {
        Ok(html) => Html(html).into_response(),
        Err(error) => {
            error!("Failed to render conformance page: {error}");
            sigma_theme::axum::internal_server_error().await
        }
    }
}

fn api_proxy<S: SessionStore + Clone + 'static>(
    session_layer: &SessionManagerLayer<S, PrivateCookie>,
    proxy_config: &ProxyConfig,
    cors_origins: Vec<HeaderValue>,
) -> anyhow::Result<Router> {
    proxy_config.extra_routes.iter().for_each(|er| {
        debug!("Adding extra route: {er:?}");
    });
    let proxy_client_https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()
        .context("Certificate roots")?
        .https_or_http()
        .enable_http1()
        .build();
    let proxy_client: ProxyClient =
        hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
            .build(proxy_client_https);
    let api_cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(cors_origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            HeaderName::from_static("x-csrf-token"),
            HeaderName::from_static("x-package-filename"),
        ])
        .allow_credentials(true);
    Ok(Router::new()
        .route(
            "/{*path}",
            delete(proxy)
                .get(proxy)
                .options(proxy_options)
                .patch(proxy)
                .post(proxy)
                .put(proxy),
        )
        .layer(
            ServiceBuilder::new()
                .layer(session_layer.clone())
                .layer(Extension(proxy_config.clone()))
                .layer(Extension(proxy_client))
                .layer(api_cors),
        ))
}

async fn verify_csrf(
    session: &Session,
    request_csrf_token: Option<&HeaderValue>,
) -> Result<(), Response> {
    let session_csrf_token: Option<String> =
        session.get(SESSION_KEY_CSRF_TOKEN).await.unwrap_or(None);
    match (request_csrf_token, session_csrf_token.as_deref()) {
        (Some(header), Some(token)) if header.as_bytes() == token.as_bytes() => Ok(()),
        _ => Err((StatusCode::FORBIDDEN, "Missing or invalid CSRF token").into_response()),
    }
}

#[debug_handler]
async fn proxy_options() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

#[debug_handler]
async fn proxy(
    Extension(proxy_config): Extension<ProxyConfig>,
    Extension(client): Extension<ProxyClient>,
    session: Session,
    jar: CookieJar,
    mut req: axum::extract::Request,
) -> Result<Response, Response> {
    let session_jwt: Option<SessionTokens> = session.get(SESSION_KEY_JWT).await.unwrap_or(None);
    let bearer = bearer_access_token(req.headers().get(AUTHORIZATION));
    let path = req.uri().path();

    let access_token = if let Some(ref tokens) = session_jwt {
        // Browser / session path: CSRF required.
        verify_csrf(&session, req.headers().get(HEADER_KEY_CSRF_TOKEN)).await?;
        ensure_proxy_admin_access(path, |roles| tokens.has_any_realm_role(roles))?;
        tokens.access_token().to_string()
    } else if let Some(token) = bearer {
        // Machine path (OIDC client-credentials): no CSRF; roles from JWT.
        ensure_proxy_admin_access(path, |roles| token_has_any_realm_role(&token, roles))?;
        token
    } else {
        return Err((StatusCode::UNAUTHORIZED, "Authentication required").into_response());
    };

    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or_else(|| req.uri().path());

    let uri = proxy_config.rewrite_uri(path_query);
    debug!("Proxy {} request to `{uri}`", req.method());
    let proxy_uri = Uri::try_from(uri.as_str()).map_err(|e| {
        debug!("Invalid proxy uri {e:?}");
        (StatusCode::BAD_REQUEST, "Invalid proxy uri").into_response()
    })?;
    let proxy_host = proxy_uri
        .host()
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid proxy uri").into_response())?;
    *req.uri_mut() = proxy_uri.clone();
    req.headers_mut().insert(
        HOST,
        HeaderValue::from_str(proxy_host)
            .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid proxy host").into_response())?,
    );

    let needle = proxy_config.cookie_name.as_str();
    let remaining_cookies = jar
        .iter()
        .filter(|h| !h.name().starts_with(needle))
        .cloned()
        .collect::<Vec<_>>();
    req.headers_mut().remove(COOKIE);
    if !remaining_cookies.is_empty() {
        let new_cookie_value = remaining_cookies
            .iter()
            .map(|c| c.encoded().to_string())
            .collect::<Vec<String>>()
            .join("; ");
        req.headers_mut().insert(
            COOKIE,
            HeaderValue::from_str(new_cookie_value.as_str()).map_err(|e| {
                debug!("Invalid cookie {e:?}");
                (StatusCode::BAD_REQUEST, "Invalid cookie").into_response()
            })?,
        );
    }

    req.headers_mut().insert(
        AUTHORIZATION,
        HeaderValue::from_bytes(format!("Bearer {access_token}").as_bytes()).map_err(|e| {
            error!("Failed to set authorization header: {e:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cannot proxy authentication",
            )
                .into_response()
        })?,
    );
    if let Some(internal_token) = sigma_pg::clients::internal::internal_token() {
        req.headers_mut().insert(
            HEADER_KEY_INTERNAL_TOKEN,
            HeaderValue::from_str(&internal_token).map_err(|e| {
                error!("Failed to set internal token header: {e:?}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Cannot proxy internal authentication",
                )
                    .into_response()
            })?,
        );
    }
    req.headers_mut().remove(HEADER_KEY_CSRF_TOKEN);

    Ok(client
        .request(req)
        .await
        .map_err(|e| {
            error!("Failed to proxy request: {e:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Cannot proxy request".to_string(),
            )
                .into_response()
        })?
        .into_response())
}

fn bearer_access_token(header: Option<&HeaderValue>) -> Option<String> {
    let raw = header?.to_str().ok()?.trim();
    let token = raw
        .strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))?
        .trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

fn ensure_proxy_admin_access(
    path: &str,
    has_role: impl FnOnce(&[&str]) -> bool,
) -> Result<(), Response> {
    if !proxy_path_requires_admin(path) {
        return Ok(());
    }
    let role_names = config::admin_realm_roles();
    let required_roles: Vec<&str> = role_names.iter().map(String::as_str).collect();
    if required_roles.is_empty() {
        return Err((StatusCode::FORBIDDEN, "Administrator access required.").into_response());
    }
    if !has_role(&required_roles) {
        return Err((StatusCode::FORBIDDEN, "Administrator access required.").into_response());
    }
    Ok(())
}

fn proxy_path_requires_admin(path: &str) -> bool {
    let path = path.trim_start_matches('/').trim_start_matches("api/");
    path.starts_with("admin")
        || path.starts_with("carts")
        || path.starts_with("orders")
        || path.starts_with("listings")
        || path.starts_with("skus")
        || path.starts_with("bills")
        || path.starts_with("contacts")
        || path.starts_with("users")
        || path.starts_with("integrations")
        || path.starts_with("v1/packages")
}

#[cfg(test)]
mod proxy_auth_tests {
    use super::{bearer_access_token, ensure_proxy_admin_access, proxy_path_requires_admin};
    use axum::http::{HeaderValue, StatusCode};
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    fn fake_jwt(payload: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let body = URL_SAFE_NO_PAD.encode(payload.as_bytes());
        format!("{header}.{body}.")
    }

    #[test]
    fn bearer_access_token_parses_header() {
        let value = HeaderValue::from_static("Bearer abc.def.ghi");
        assert_eq!(
            bearer_access_token(Some(&value)).as_deref(),
            Some("abc.def.ghi")
        );
        assert!(bearer_access_token(None).is_none());
        assert!(bearer_access_token(Some(&HeaderValue::from_static("Basic x"))).is_none());
    }

    #[test]
    fn packages_path_requires_admin() {
        assert!(proxy_path_requires_admin("/api/v1/packages"));
        assert!(proxy_path_requires_admin("/v1/packages"));
        assert!(!proxy_path_requires_admin("/v1/channels"));
    }

    #[test]
    fn ensure_proxy_admin_rejects_missing_role() {
        let err = ensure_proxy_admin_access("/api/v1/packages", |_| false).unwrap_err();
        assert_eq!(err.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn ensure_proxy_admin_accepts_matching_role() {
        let token = fake_jwt(r#"{"realm_access":{"roles":["sigma-admin"]}}"#);
        assert!(
            ensure_proxy_admin_access("/api/v1/packages", |roles| {
                crate::auth::token_has_any_realm_role(&token, roles)
            })
            .is_ok()
        );
    }
}

fn security_headers(router: Router) -> Router {
    let csp = if config::is_production() {
        sigma_theme::public_html_csp_production("", false)
    } else {
        sigma_theme::public_html_csp("", false)
    };

    let mut router = router
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("content-security-policy"),
            HeaderValue::from_str(&csp).expect("valid csp"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("cross-origin-opener-policy"),
            HeaderValue::from_static("same-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("permissions-policy"),
            HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
        ));

    if config::is_production() {
        router = router.layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
        ));
    }

    router
}

pub(crate) struct AppBuildOptions {
    pub remaining_secs_threshold: u64,
    pub app_config: AppConfigurationState,
    pub registration: Option<RegistrationDeps>,
    pub profile: Option<ProfileDeps>,
    pub admin: Option<AdminDeps>,
}

pub(crate) fn app<S: SessionStore + Clone + 'static>(
    oidc_client: OIDCClient,
    session_layer: &SessionManagerLayer<S, PrivateCookie>,
    proxy_config: &ProxyConfig,
    pool: PgPool,
    options: AppBuildOptions,
) -> Result<Router> {
    let AppBuildOptions {
        remaining_secs_threshold,
        app_config,
        registration,
        profile,
        admin,
    } = options;
    let files_root = config::files_dir();
    let files_root_path = Path::new(&files_root);
    let spa_apps = walk_dir(&files_root)?;
    let register_oidc = oidc_client.clone();
    let cors_origins = app_config.login_app_settings.cors_origins().to_vec();
    let mut app = Router::new()
        .route("/geo/regions", get(crate::geo::regions_handler))
        .merge(health_routes(pool.clone()))
        .nest(
            "/api",
            api_proxy(session_layer, proxy_config, cors_origins)?,
        )
        .nest("/app", health_routes(pool.clone()))
        .nest(
            "/auth",
            auth_routes(
                oidc_client,
                session_layer,
                remaining_secs_threshold,
                app_config,
            ),
        )
        .merge(sigma_theme::axum::router())
        .merge(themed_page_routes());

    if let Some(RegistrationDeps {
        settings,
        admin,
        adapter,
    }) = registration
    {
        app = app.merge(register_routes(
            settings,
            admin,
            adapter,
            register_oidc,
            sigma_human_check::HumanCheck::from_env(),
            session_layer,
        ));
    }

    if let Some(ProfileDeps { settings, admin }) = profile {
        app = app.merge(crate::auth::profile_routes(settings, admin, session_layer));
    }

    if let Some(AdminDeps { admin }) = admin {
        app = app.merge(crate::auth::admin_routes(admin, session_layer));
    }

    app = mount_themed_app_assets(app, files_root_path);

    for spa_app in spa_apps {
        let route_suffix = spa_app
            .strip_prefix(files_root_path)
            .unwrap_or(spa_app.as_path());
        let uri_path = if route_suffix.as_os_str().is_empty() {
            "/".to_string()
        } else {
            format!("/{}", route_suffix.to_string_lossy().replace('\\', "/"))
        };
        let fs_path = spa_app.as_path();
        debug!("Serving route {uri_path} from folder {fs_path:?}");
        let mut fallback = spa_app.clone();
        fallback.push("index.html");
        let serve_dir = ServeDir::new(fs_path).not_found_service(ServeFile::new(fallback));

        if uri_path == "/" {
            app = app.route_service("/", serve_dir);
        } else {
            app = app.nest_service(&uri_path, serve_dir);
        }
    }

    Ok(security_headers(
        app.fallback(sigma_theme::axum::not_found)
            .layer(sigma_theme::axum::catch_panic_layer()),
    ))
}
