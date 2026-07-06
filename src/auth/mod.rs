mod admin;
pub mod allowlist;
mod callback;
mod conformance;
mod csrftoken;
mod keycloak_admin;
mod login;
mod logout;
mod oidcclient;
mod profile;
mod refresh;
mod register;
mod registration_adapter;
mod status;

use std::time::SystemTime;

pub use login::LoginAppSettings;
pub use logout::LogoutAppSettings;
pub use logout::LogoutBehavior;
pub use oidcclient::OIDCClient;
pub(crate) use profile::ProfileDeps;
pub use register::{RegistrationAppSettings, RegistrationDeps};
pub(crate) use registration_adapter::RegistrationAdapter;

pub(crate) use admin::AdminDeps;
pub(crate) use keycloak_admin::{KeycloakAdmin, ProfileInput};

use axum::http::Method;
use axum::{
    Extension, Router,
    extract::FromRef,
    routing::{get, post},
};
use tower::ServiceBuilder;
use tower_http::cors::{AllowOrigin, CorsLayer};

use tower_sessions::{SessionManagerLayer, SessionStore, service::PrivateCookie};

use openidconnect::{
    AccessToken, CsrfToken, Nonce, PkceCodeVerifier, RefreshToken, core::CoreIdToken, url::Url,
};
use serde::{Deserialize, Serialize};

use self::{
    admin::{admin_user_approve, admin_user_detail, admin_user_list, admin_user_reset_password},
    callback::callback,
    csrftoken::csrftoken,
    login::login,
    logout::{logout, logout_callback},
    profile::{profile_form, profile_submit},
    refresh::{RefreshLockManager, refresh},
    register::{register_form, register_submit, register_success, register_verified},
    status::status,
};

#[derive(Debug, Clone)]
pub(crate) struct AuthorizeData {
    auth_url: String,
    csrf_token: String,
    nonce: String,
    pkce_verifier: String,
}

impl AuthorizeData {
    pub(crate) fn new(
        auth_url: Url,
        csrf_token: CsrfToken,
        nonce: Nonce,
        pkce_verifier: PkceCodeVerifier,
    ) -> Self {
        Self {
            auth_url: auth_url.to_string(),
            csrf_token: csrf_token.secret().clone(),
            nonce: nonce.secret().clone(),
            pkce_verifier: pkce_verifier.secret().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SessionTokens {
    access_token: String,
    refresh_token: Option<String>,
    id_token: String,
    expires_at: SystemTime,
    refresh_expires_at: SystemTime,
}

impl SessionTokens {
    pub(crate) fn new(
        access_token: &AccessToken,
        refresh_token: Option<&RefreshToken>,
        id_token: &CoreIdToken,
        expires_at: SystemTime,
        refresh_expires_at: SystemTime,
    ) -> Self {
        Self {
            access_token: access_token.secret().to_string(),
            refresh_token: refresh_token.map(|r| r.secret().to_string()),
            id_token: id_token.to_string(),
            expires_at,
            refresh_expires_at,
            // expires_at: now + Duration::from_secs(expires_in as u64),
            // refresh_expires_at: now + Duration::from_secs(refresh_expires_in as u64),
        }
    }

    pub(crate) fn access_token(&self) -> &str {
        &self.access_token
    }

    pub(crate) fn refresh_token(&self) -> Option<String> {
        self.refresh_token.as_ref().cloned()
    }

    pub(crate) fn ttl_gt(&self, threshold: u64) -> bool {
        let now = SystemTime::now();
        self.expires_at
            .duration_since(now)
            .map(|st| st.as_secs())
            .unwrap_or_default()
            > threshold
    }

    /// Display name from the stored ID token (preferred_username, name, or email).
    pub(crate) fn display_name(&self) -> Option<String> {
        display_name_from_id_token(&self.id_token)
    }

    /// Email from the stored ID token.
    pub(crate) fn email(&self) -> Option<String> {
        email_from_id_token(&self.id_token)
    }

    /// Subject (`sub`) from the stored ID token.
    pub(crate) fn subject(&self) -> Option<String> {
        jwt_claims(&self.id_token)?
            .get("sub")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    /// Whether the stored tokens include any of the given realm roles.
    ///
    /// Keycloak's default `roles` client scope puts `realm_access` on the access
    /// token, not the ID token, so both are checked.
    pub(crate) fn has_any_realm_role(&self, roles: &[&str]) -> bool {
        token_has_any_realm_role(&self.access_token, roles)
            || token_has_any_realm_role(&self.id_token, roles)
    }
}

fn token_has_any_realm_role(token: &str, roles: &[&str]) -> bool {
    let Some(claims) = jwt_claims(token) else {
        return false;
    };
    let Some(role_values) = claims
        .get("realm_access")
        .and_then(|access| access.get("roles"))
        .and_then(|value| value.as_array())
    else {
        return false;
    };
    role_values
        .iter()
        .any(|value| value.as_str().is_some_and(|role| roles.contains(&role)))
}

fn jwt_claims(token: &str) -> Option<serde_json::Value> {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod realm_role_tests {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;

    use super::token_has_any_realm_role;

    fn fake_jwt(payload: &str) -> String {
        let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let body = URL_SAFE_NO_PAD.encode(payload.as_bytes());
        format!("{header}.{body}.")
    }

    #[test]
    fn realm_roles_are_read_from_access_token_payload() {
        let token = fake_jwt(r#"{"realm_access":{"roles":["sigma-admin"]}}"#);
        assert!(token_has_any_realm_role(&token, &["sigma-admin"]));
        assert!(!token_has_any_realm_role(&token, &["other-role"]));
    }
}

fn display_name_from_id_token(id_token: &str) -> Option<String> {
    let claims = jwt_claims(id_token)?;
    if let Some(username) = claims
        .get("preferred_username")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Some(username.to_string());
    }
    if let Some(name) = claims
        .get("name")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Some(name.to_string());
    }
    claims
        .get("email")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|email| {
            email
                .split('@')
                .next()
                .filter(|local| !local.is_empty())
                .unwrap_or(email)
                .to_string()
        })
}

fn email_from_id_token(id_token: &str) -> Option<String> {
    jwt_claims(id_token)?
        .get("email")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LoginCallbackSessionParameters {
    app_uri: String,
    nonce: String,
    csrf_token: String,
    pkce_verifier: String,
    redirect_uri: String,
    scopes: String,
}

#[derive(Clone)]
pub(crate) struct AppConfigurationState {
    pub(crate) login_app_settings: LoginAppSettings,
    pub(crate) logout_app_settings: LogoutAppSettings,
}

impl FromRef<AppConfigurationState> for LoginAppSettings {
    fn from_ref(app_state: &AppConfigurationState) -> Self {
        app_state.login_app_settings.clone()
    }
}

impl FromRef<AppConfigurationState> for LogoutAppSettings {
    fn from_ref(app_state: &AppConfigurationState) -> LogoutAppSettings {
        app_state.logout_app_settings.clone()
    }
}

pub(crate) fn auth_routes<S: SessionStore + Clone + 'static>(
    oidc_client: OIDCClient,
    session_layer: &SessionManagerLayer<S, PrivateCookie>,
    remaining_secs_threshold: u64,
    app_config: AppConfigurationState,
) -> Router {
    let rlm = RefreshLockManager::new(remaining_secs_threshold);
    let callback_route = if crate::config::conformance_mode() {
        get(callback).post(conformance::callback_form_post)
    } else {
        get(callback)
    };
    let status_cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(
            app_config.login_app_settings.cors_origins().to_vec(),
        ))
        .allow_methods([Method::GET, Method::OPTIONS])
        .allow_credentials(true);

    let mut router = Router::new()
        .route(
            "/login",
            get(login).layer(ServiceBuilder::new().layer(Extension(oidc_client.clone()))),
        )
        .route(
            "/callback",
            callback_route.layer(ServiceBuilder::new().layer(Extension(oidc_client.clone()))),
        )
        .route(
            "/refresh",
            post(refresh).layer(
                ServiceBuilder::new()
                    .layer(Extension(rlm))
                    .layer(Extension(oidc_client.clone())),
            ),
        )
        .route("/csrftoken", post(csrftoken))
        .route("/status", get(status).layer(status_cors))
        .route("/logout", get(logout))
        .route("/logoutcallback", get(logout_callback))
        .layer(session_layer.clone())
        .with_state(app_config);

    if crate::config::conformance_mode() {
        router = router.route(
            "/conformance/discover",
            get(conformance::discover).layer(Extension(oidc_client)),
        );
    }

    router
}

pub(crate) fn register_routes<S: SessionStore + Clone + 'static>(
    registration_settings: RegistrationAppSettings,
    keycloak_admin: KeycloakAdmin,
    registration_adapter: RegistrationAdapter,
    oidc_client: OIDCClient,
    session_layer: &SessionManagerLayer<S, PrivateCookie>,
) -> Router {
    Router::new()
        .route(
            "/register",
            get(register_form).post(register_submit).layer(
                ServiceBuilder::new()
                    .layer(Extension(keycloak_admin.clone()))
                    .layer(Extension(registration_adapter)),
            ),
        )
        .route(
            "/register/verified/{user_id}",
            get(register_verified).layer(Extension(keycloak_admin)),
        )
        .route("/register/success", get(register_success))
        .layer(Extension(oidc_client))
        .layer(session_layer.clone())
        .with_state(registration_settings)
}

pub(crate) fn profile_routes<S: SessionStore + Clone + 'static>(
    profile_settings: RegistrationAppSettings,
    keycloak_admin: KeycloakAdmin,
    session_layer: &SessionManagerLayer<S, PrivateCookie>,
) -> Router {
    Router::new()
        .route(
            "/profile",
            get(profile_form)
                .post(profile_submit)
                .layer(Extension(keycloak_admin)),
        )
        .layer(session_layer.clone())
        .with_state(profile_settings)
}

pub(crate) fn admin_routes<S: SessionStore + Clone + 'static>(
    keycloak_admin: KeycloakAdmin,
    session_layer: &SessionManagerLayer<S, PrivateCookie>,
) -> Router {
    Router::new()
        .route(
            "/admin/users",
            get(admin_user_list).layer(Extension(keycloak_admin.clone())),
        )
        .route(
            "/admin/users/{user_id}",
            get(admin_user_detail).layer(Extension(keycloak_admin.clone())),
        )
        .route(
            "/admin/users/{user_id}/approve",
            post(admin_user_approve).layer(Extension(keycloak_admin.clone())),
        )
        .route(
            "/admin/users/{user_id}/reset-password",
            post(admin_user_reset_password).layer(Extension(keycloak_admin)),
        )
        .layer(session_layer.clone())
}

pub(crate) fn random_alphanumeric_string(length: usize) -> String {
    rand::Rng::sample_iter(rand::rng(), &rand::distr::Alphanumeric)
        .take(length)
        .map(char::from)
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{Router, body::Body, http::HeaderValue};
    use base64::prelude::*;
    use chrono::Utc;
    use http_body_util::BodyExt;
    use hyper::{
        Request,
        header::{COOKIE, LOCATION, SET_COOKIE},
    };
    use once_cell::sync::Lazy;
    use openidconnect::{
        AccessToken, Audience, AuthUrl, EmptyAdditionalClaims, EmptyAdditionalProviderMetadata,
        EmptyExtraTokenFields, EndUserEmail, EndUserUsername, IdToken, IssuerUrl, JsonWebKeyId,
        JsonWebKeySetUrl, Nonce, PrivateSigningKey, RefreshToken, ResponseTypes, Scope,
        StandardClaims, SubjectIdentifier, TokenUrl, UserInfoUrl,
        core::{
            CoreClaimName, CoreIdToken, CoreIdTokenClaims, CoreIdTokenFields, CoreJsonWebKeySet,
            CoreJwsSigningAlgorithm, CoreProviderMetadata, CoreResponseType,
            CoreRsaPrivateSigningKey, CoreSubjectIdentifierType, CoreTokenResponse, CoreTokenType,
        },
    };
    use serde_json::json;
    use tower::{Service, ServiceExt};
    use tracing_log::LogTracer;
    use tracing_subscriber::filter::EnvFilter;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use tower_sessions::{SessionManagerLayer, service::PrivateCookie};

    use tower_sessions_sqlx_store::PostgresStore;

    use crate::session::{SessionSetup, postgres_pool, session_store};

    use super::allowlist::UriAllowlist;
    use super::{
        AppConfigurationState, LoginAppSettings, OIDCClient, auth_routes,
        logout::{LogoutAppSettings, LogoutBehavior},
        random_alphanumeric_string,
    };

    static GLOBAL_LOGGER_SETUP: Lazy<Arc<bool>> = Lazy::new(|| {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::builder()
                    .with_default_directive(
                        "sigma_identity=debug"
                            .parse()
                            .expect("Directive should parse"),
                    )
                    .from_env_lossy(),
            )
            .init();
        let _ = LogTracer::init();
        Arc::new(true)
    });

    const TEST_RSA_PEM: &str = include_str!("../../test-fixtures/rsa-private.pem");

    fn test_rsa_pem() -> &'static str {
        TEST_RSA_PEM
    }

    fn oidc_body(issuer: &str) -> String {
        let provider_metadata = CoreProviderMetadata::new(
            // Parameters required by the OpenID Connect Discovery spec.
            IssuerUrl::new(issuer.to_string()).expect("Invalid issuer URL"),
            AuthUrl::new(format!("{issuer}/authorize")).expect("Auth URL is invalid"),
            // Use the JsonWebKeySet struct to serve the JWK Set at this URL.
            JsonWebKeySetUrl::new(format!("{issuer}/.well-known/jwks.json"))
                .expect("JWK Set URL is invalid"),
            // Supported response types (flows).
            vec![
                // Recommended: support the code flow.
                ResponseTypes::new(vec![CoreResponseType::Code]),
                // Optional: support the implicit flow.
                //ResponseTypes::new(vec![CoreResponseType::Token, CoreResponseType::IdToken]), // Other flows including hybrid flows may also be specified here.
            ],
            // For user privacy, the Pairwise subject identifier type is preferred. This prevents
            // distinct relying parties (clients) from knowing whether their users represent the same
            // real identities. This identifier type is only useful for relying parties that don't
            // receive the 'email', 'profile' or other personally-identifying scopes.
            // The Public subject identifier type is also supported.
            vec![CoreSubjectIdentifierType::Pairwise],
            // Support the RS256 signature algorithm.
            vec![CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256],
            // OpenID Connect Providers may supply custom metadata by providing a struct that
            // implements the AdditionalProviderMetadata trait. This requires manually using the
            // generic ProviderMetadata struct rather than the CoreProviderMetadata type alias,
            // however.
            EmptyAdditionalProviderMetadata {},
        )
        // Specify the token endpoint (required for the code flow).
        .set_token_endpoint(Some(
            TokenUrl::new(format!("{issuer}/token")).expect("Invalid token URL"),
        ))
        // Recommended: support the UserInfo endpoint.
        .set_userinfo_endpoint(Some(
            UserInfoUrl::new(format!("{issuer}/userinfo")).expect("userinfo endpoint is invalid"),
        ))
        // Recommended: specify the supported scopes.
        .set_scopes_supported(Some(vec![
            Scope::new("openid".to_string()),
            Scope::new("email".to_string()),
            Scope::new("profile".to_string()),
        ]))
        // Recommended: specify the supported ID token claims.
        .set_claims_supported(Some(vec![
            // Providers may also define an enum instead of using CoreClaimName.
            CoreClaimName::new("sub".to_string()),
            CoreClaimName::new("aud".to_string()),
            CoreClaimName::new("email".to_string()),
            CoreClaimName::new("email_verified".to_string()),
            CoreClaimName::new("exp".to_string()),
            CoreClaimName::new("iat".to_string()),
            CoreClaimName::new("iss".to_string()),
            CoreClaimName::new("name".to_string()),
            CoreClaimName::new("given_name".to_string()),
            CoreClaimName::new("family_name".to_string()),
            CoreClaimName::new("picture".to_string()),
            CoreClaimName::new("locale".to_string()),
        ]));

        serde_json::to_string(&provider_metadata).expect("Serialize ProviderMetadata")
    }

    fn oidc_keys() -> String {
        let rsa_pem = test_rsa_pem();
        let jwks = CoreJsonWebKeySet::new(vec![
            // RSA keys may also be constructed directly using CoreJsonWebKey::new_rsa(). Providers
            // aiming to support other key types may provide their own implementation of the
            // JsonWebKey trait or submit a PR to add the desired support to this crate.
            CoreRsaPrivateSigningKey::from_pem(
                rsa_pem,
                Some(JsonWebKeyId::new("key1".to_string())),
            )
            .expect("Invalid RSA private key")
            .as_verification_key(),
        ]);

        serde_json::to_string(&jwks).expect("Serialize JSON Web Key Set")
    }

    fn id_token(
        issuer: &str,
        client_id: &str,
        nonce: &str,
        access_token: &AccessToken,
    ) -> IdToken<
        EmptyAdditionalClaims,
        openidconnect::core::CoreGenderClaim,
        openidconnect::core::CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
    > {
        let rsa_pem = test_rsa_pem();
        CoreIdToken::new(
            CoreIdTokenClaims::new(
                // Specify the issuer URL for the OpenID Connect Provider.
                IssuerUrl::new(issuer.to_string()).expect("Invalid issuer URL"),
                // The audience is usually a single entry with the client ID of the client for whom
                // the ID token is intended. This is a required claim.
                vec![Audience::new(client_id.to_string())],
                // The ID token expiration is usually much shorter than that of the access or refresh
                // tokens issued to clients.
                Utc::now() + chrono::Duration::seconds(300),
                // The issue time is usually the current time.
                Utc::now(),
                // Set the standard claims defined by the OpenID Connect Core spec.
                StandardClaims::new(
                    // Stable subject identifiers are recommended in place of e-mail addresses or other
                    // potentially unstable identifiers. This is the only required claim.
                    SubjectIdentifier::new("5f83e0ca-2b8e-4e8c-ba0a-f80fe9bc3632".to_string()),
                )
                // Optional: specify the user's e-mail address. This should only be provided if the
                // client has been granted the 'profile' or 'email' scopes.
                .set_email(Some(EndUserEmail::new("bob@example.com".to_string())))
                .set_preferred_username(Some(EndUserUsername::new("bob".to_string())))
                // Optional: specify whether the provider has verified the user's e-mail address.
                .set_email_verified(Some(true)),
                // OpenID Connect Providers may supply custom claims by providing a struct that
                // implements the AdditionalClaims trait. This requires manually using the
                // generic IdTokenClaims struct rather than the CoreIdTokenClaims type alias,
                // however.
                EmptyAdditionalClaims {},
            )
            .set_nonce(Some(Nonce::new(nonce.to_string()))),
            // The private key used for signing the ID token. For confidential clients (those able
            // to maintain a client secret), a CoreHmacKey can also be used, in conjunction
            // with one of the CoreJwsSigningAlgorithm::HmacSha* signing algorithms. When using an
            // HMAC-based signing algorithm, the UTF-8 representation of the client secret should
            // be used as the HMAC key.
            &CoreRsaPrivateSigningKey::from_pem(
                rsa_pem,
                Some(JsonWebKeyId::new("key1".to_string())),
            )
            .expect("Invalid RSA private key"),
            // Uses the RS256 signature algorithm. This crate supports any RS*, PS*, or HS*
            // signature algorithm.
            CoreJwsSigningAlgorithm::RsaSsaPkcs1V15Sha256,
            // When returning the ID token alongside an access token (e.g., in the Authorization Code
            // flow), it is recommended to pass the access token here to set the `at_hash` claim
            // automatically.
            Some(access_token),
            // When returning the ID token alongside an authorization code (e.g., in the implicit
            // flow), it is recommended to pass the authorization code here to set the `c_hash` claim
            // automatically.
            None,
        )
        .expect("Invalid ID token")
    }

    fn oidc_token(issuer: &str, client_id: &str, nonce: &str) -> String {
        let opaque_access_token = random_alphanumeric_string(32);
        let access_token = AccessToken::new(opaque_access_token);

        let id_token = id_token(issuer, client_id, nonce, &access_token);

        let mut token_response = CoreTokenResponse::new(
            access_token,
            CoreTokenType::Bearer,
            CoreIdTokenFields::new(Some(id_token), EmptyExtraTokenFields {}),
        );
        let exp = Utc::now() + chrono::Duration::seconds(600);
        let exp_timestamp = exp.timestamp();
        let refresh_token = json!({"exp": exp_timestamp}).to_string();
        let refresh_token = format!("A.{}.S", BASE64_STANDARD.encode(refresh_token));
        let refresh_token = RefreshToken::new(refresh_token);
        let d = core::time::Duration::from_secs(13);
        token_response.set_expires_in(Some(&d));
        token_response.set_refresh_token(Some(refresh_token));
        serde_json::to_string(&token_response).expect("Serialize token response")
    }

    pub struct MockSetup {
        client_id: String,
        cookie_name: String,
        issuer_url: String,
        mock_server: MockServer,
        oidc_client: OIDCClient,
        session_layer: SessionManagerLayer<PostgresStore, PrivateCookie>,
    }

    impl MockSetup {
        pub(crate) async fn new() -> Self {
            let _b = GLOBAL_LOGGER_SETUP.clone();
            let mock_server = MockServer::start().await;
            let issuer_url = format!("{}/testing-issuer", mock_server.uri());
            let cookie_name = format!("{}.sid", random_alphanumeric_string(8));
            let client_id = "test-client".to_string();
            let client_secret: String = random_alphanumeric_string(20);
            Mock::given(method("GET"))
                .and(path("/testing-issuer/.well-known/openid-configuration"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_raw(oidc_body(&issuer_url), "application/json"),
                )
                .mount(&mock_server)
                .await;
            Mock::given(method("GET"))
                .and(path("/testing-issuer/.well-known/jwks.json"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_raw(oidc_keys(), "application/json"),
                )
                .mount(&mock_server)
                .await;
            let auth_url = format!("{}/authorize", mock_server.uri());
            let oidc_client = OIDCClient::build(
                &issuer_url,
                &client_id,
                &client_secret,
                Some(auth_url),
                None,
                None,
                None,
            )
            .await
            .expect("OIDCClient creation failed");

            let session_secret: String = random_alphanumeric_string(64);
            let database_url = crate::config::var_optional("TEST_DATABASE_URL")
                .unwrap_or_else(|| sigma_pg::DEFAULT_DATABASE_URL.to_string());
            let pg_pool = postgres_pool(&database_url)
                .await
                .expect("PostgreSQL required for tests");
            let session_store = session_store(pg_pool)
                .await
                .expect("session store migration failed");
            let session_setup = SessionSetup {
                secret: session_secret.clone(),
                cookie_name: cookie_name.clone(),
                cookie_path: "/".to_string(),
                ttl: Some(time::Duration::new(300, 0)),
                secure_cookie: true,
                same_site: crate::SameSiteSetting::Strict,
            };
            let session_layer = session_setup
                .get_session_layer(session_store.clone())
                .expect("Session setup failed");

            Self {
                cookie_name,
                client_id,
                issuer_url,
                mock_server,
                oidc_client,
                session_layer,
            }
        }

        pub fn router(&self) -> Router {
            let app_config = AppConfigurationState {
                login_app_settings: LoginAppSettings::new(
                    vec![
                        "http://example.com".to_string(),
                        "http://example.org/my/app/*".to_string(),
                    ],
                    vec!["http://example.com/auth/callback".to_string()],
                ),
                logout_app_settings: LogoutAppSettings {
                    client_id: self.client_id.clone(),
                    logout_uri: format!("{}/logout", self.mock_server.uri()),
                    _behavior: LogoutBehavior::FrontChannelLogoutWithIdToken,
                    allowed_app_uris: UriAllowlist::new(vec![
                        "http://logout.example.com".to_string(),
                        "http://example.org/it/index".to_string(),
                    ]),
                    allowed_oidc_redirect_uris: UriAllowlist::new(vec![
                        "http://example.com/auth/logoutcallback".to_string(),
                    ]),
                },
            };
            Router::new().nest(
                "/auth",
                auth_routes(
                    self.oidc_client.clone(),
                    &self.session_layer,
                    20,
                    app_config,
                ),
            )
        }

        pub async fn setup_id_token_nonce(&self, header: &HeaderValue) {
            let nonce: String = header
                .to_str()
                .unwrap()
                .split('&')
                .find(|s| s.starts_with("nonce"))
                .expect("Redirect uri should have nonce")
                .split('=')
                .skip(1)
                .take(1)
                .collect();
            Mock::given(method("POST"))
                .and(path("/testing-issuer/token"))
                .respond_with(ResponseTemplate::new(200).set_body_raw(
                    oidc_token(&self.issuer_url, &self.client_id, &nonce),
                    "application/json",
                ))
                .mount(&self.mock_server)
                .await;
        }

        pub async fn setup_refresh_token_response(&self, nonce: &str) {
            Mock::given(method("POST"))
                .and(path("/testing-issuer/token"))
                .respond_with(ResponseTemplate::new(200).set_body_raw(
                    oidc_token(&self.issuer_url, &self.client_id, nonce),
                    "application/json",
                ))
                .mount(&self.mock_server)
                .await;
        }

        pub async fn setup_authenticated_state(&self, app: &mut Router) -> String {
            // Call login to create a session
            let login_request = Request::builder()
                .uri("/auth/login?app_uri=http%3A%2F%2Fexample.com&redirect_uri=http%3A%2F%2Fexample.com%2Fauth%2Fcallback&scope=openid")
                .body(Body::empty())
                .unwrap();
            let login_response = ServiceExt::<Request<Body>>::ready(app)
                .await
                .unwrap()
                .call(login_request)
                .await
                .unwrap();
            // Assert redirect
            assert_eq!(
                303,
                login_response.status().as_u16(),
                "No redirect: {}",
                String::from_utf8(
                    login_response
                        .into_body()
                        .collect()
                        .await
                        .unwrap()
                        .to_bytes()
                        .to_vec()
                )
                .unwrap_or_else(|e| format!("utf8 error: {e}"))
            );
            let location = login_response
                .headers()
                .get(LOCATION)
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();
            // Extract state from redirect uri
            let state: String = location
                .split('&')
                .find(|s| s.starts_with("state"))
                .expect("Redirect uri should have nonce")
                .split('=')
                .skip(1)
                .take(1)
                .collect();
            // Extract nonce from redirect uri
            let nonce: String = location
                .split('&')
                .find(|s| s.starts_with("nonce"))
                .expect("Redirect uri should have nonce")
                .split('=')
                .skip(1)
                .take(1)
                .collect();

            // Extract session cookie
            let session_cookie = login_response
                .headers()
                .get(SET_COOKIE)
                .expect("Login response should have a session cookie")
                .to_str()
                .expect("Session cookie should be a string")
                .split(';')
                .find(|s| s.starts_with(&self.cookie_name))
                .expect("Login response should have a session cookie");

            // prepare mock server for token exchange
            let code = random_alphanumeric_string(32);
            Mock::given(method("POST"))
                .and(path("/testing-issuer/token"))
                .respond_with(ResponseTemplate::new(200).set_body_raw(
                    oidc_token(&self.issuer_url, &self.client_id, &nonce),
                    "application/json",
                ))
                .mount(&self.mock_server)
                .await;

            // Run callback request
            let callback_request = Request::builder()
                .uri(format!("/auth/callback?code={code}&state={state}",))
                .header(COOKIE, session_cookie)
                .body(Body::empty())
                .unwrap();
            let callback_response = ServiceExt::<Request<Body>>::ready(app)
                .await
                .unwrap()
                .call(callback_request)
                .await
                .unwrap();
            // Extract session cookie for authenticated state
            let authenticated_cookie = callback_response
                .headers()
                .get(SET_COOKIE)
                .expect("Callback response should have a session cookie")
                .to_str()
                .expect("Session cookie should be a string")
                .split(';')
                .find(|s| s.starts_with(&self.cookie_name))
                .expect("Callback response should have a session cookie");

            assert_eq!(
                callback_response.status(),
                303,
                "Response not ok: {}",
                String::from_utf8(
                    callback_response
                        .into_body()
                        .collect()
                        .await
                        .unwrap()
                        .to_bytes()
                        .to_vec()
                )
                .unwrap_or_else(|e| format!("utf8 error: {e}"))
            );

            authenticated_cookie.to_string()
        }
    }
}
