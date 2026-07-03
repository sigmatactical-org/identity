use std::{
    borrow::Cow,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use oauth2::{AuthType, EndpointMaybeSet, EndpointNotSet, EndpointSet, basic::BasicTokenType};
use openidconnect::{
    AccessTokenHash, AuthUrl, AuthorizationCode, ClaimsVerificationError, ClientId, ClientSecret,
    CsrfToken, EmptyAdditionalClaims, EmptyExtraTokenFields, IdTokenFields, IssuerUrl, JsonWebKeySetUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope,
    SignatureVerificationError, StandardTokenResponse, TokenResponse, TokenUrl,
    core::{
        CoreAuthenticationFlow, CoreClient, CoreGenderClaim, CoreIdToken, CoreIdTokenClaims,
        CoreIdTokenVerifier, CoreJsonWebKeySet, CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm, CoreProviderMetadata,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tracing::{debug, trace};
use url::Url;

use crate::config;

use super::{AuthorizeData, SessionTokens, callback::TokenExchangeData};

type KeycloakTokenResponse = StandardTokenResponse<
    IdTokenFields<
        EmptyAdditionalClaims,
        EmptyExtraTokenFields,
        CoreGenderClaim,
        CoreJweContentEncryptionAlgorithm,
        CoreJwsSigningAlgorithm,
    >,
    BasicTokenType,
>;

type OidcCoreClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

#[derive(Clone)]
struct OidcInner {
    client: OidcCoreClient,
    metadata: CoreProviderMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExpFieldInJWT {
    pub(crate) exp: u64,
}

pub struct AuthorizeRequestData {
    pub redirect_uri: String,
    pub scope: String,
    pub state: String,
    pub ui_locales: Option<String>,
    pub prompt: Option<String>,
    pub kc_idp_hint: Option<String>,
}

#[derive(Clone)]
pub struct OIDCClient {
    http_client: oauth2::reqwest::Client,
    issuer_url: String,
    client_id: String,
    client_secret: String,
    auth_url_override: Option<String>,
    discovery_issuer_url: Option<String>,
    token_endpoint_override: Option<String>,
    jwks_uri_override: Option<String>,
    inner: Arc<RwLock<Option<Arc<OidcInner>>>>,
}

fn jwt_exp(jwt: &str) -> Result<u64> {
    let payload = jwt.split('.').nth(1).unwrap_or_default();
    let payload = STANDARD_NO_PAD.decode(payload)?;
    let payload = String::from_utf8(payload)?;
    let jwtdecoded: ExpFieldInJWT = serde_json::from_str(payload.as_str())?;
    Ok(jwtdecoded.exp)
}

fn normalize_issuer(issuer: &str) -> String {
    issuer.trim_end_matches('/').to_string()
}

fn decode_jwt_payload(id_token: &str) -> Result<Value> {
    let payload = id_token.split('.').nth(1).unwrap_or_default();
    let payload = STANDARD_NO_PAD.decode(payload)?;
    Ok(serde_json::from_slice(&payload)?)
}

fn validate_userinfo_subject(
    payload: &Value,
    expected: &openidconnect::SubjectIdentifier,
) -> Result<()> {
    let sub = payload
        .get("sub")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Userinfo response missing sub claim"))?;
    if sub != expected.as_str() {
        return Err(anyhow!("Userinfo sub does not match ID token sub"));
    }
    Ok(())
}

fn token_response_to_session_tokens(
    token_response: &KeycloakTokenResponse,
) -> Result<SessionTokens> {
    let id_token = token_response
        .id_token()
        .ok_or_else(|| anyhow!("Server did not return an ID token"))?;

    let expires_at = if let Ok(exp) = jwt_exp(token_response.access_token().secret()) {
        UNIX_EPOCH + Duration::from_secs(exp)
    } else {
        token_response
            .expires_in()
            .map(|exp| SystemTime::now() + exp)
            .unwrap_or_else(SystemTime::now)
    };

    let refresh_expires_at = token_response
        .refresh_token()
        .and_then(|rt| jwt_exp(rt.secret()).ok())
        .map(|exp| UNIX_EPOCH + Duration::from_secs(exp))
        .unwrap_or_else(SystemTime::now);

    Ok(SessionTokens::new(
        token_response.access_token(),
        token_response.refresh_token(),
        id_token,
        expires_at,
        refresh_expires_at,
    ))
}

impl OIDCClient {
    pub(crate) async fn build(
        issuer_url: &str,
        client_id: &str,
        client_secret: &str,
        authorization_endpoint: Option<String>,
        discovery_issuer_url: Option<String>,
        token_endpoint_override: Option<String>,
        jwks_uri_override: Option<String>,
    ) -> Result<Self> {
        let http_client = {
            let danger_accept_invalid_certs = config::danger_accept_invalid_certs()?;
            oauth2::reqwest::ClientBuilder::new()
                .danger_accept_invalid_certs(danger_accept_invalid_certs)
                .redirect(oauth2::reqwest::redirect::Policy::none())
                .build()
                .context("Failed to create http client")?
        };

        Ok(Self {
            http_client,
            issuer_url: issuer_url.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            auth_url_override: authorization_endpoint,
            discovery_issuer_url,
            token_endpoint_override,
            jwks_uri_override,
            inner: Arc::new(RwLock::new(None)),
        })
    }

    pub(crate) async fn refresh_provider_metadata(&self) -> Result<String> {
        self.invalidate().await;
        let inner = self.ensure_inner().await?;
        Ok(inner.metadata.issuer().url().as_str().to_string())
    }

    async fn invalidate(&self) {
        *self.inner.write().await = None;
    }

    async fn ensure_inner(&self) -> Result<Arc<OidcInner>> {
        if let Some(inner) = self.inner.read().await.clone() {
            return Ok(inner);
        }

        let mut guard = self.inner.write().await;
        if let Some(inner) = guard.clone() {
            return Ok(inner);
        }

        let metadata = self.discover_metadata().await?;
        let inner = Arc::new(self.build_inner(metadata).await?);
        *guard = Some(inner.clone());
        Ok(inner)
    }

    async fn discover_metadata(&self) -> Result<CoreProviderMetadata> {
        let discovery_issuer = if let Some(resource) = config::oidc_webfinger_resource() {
            self.webfinger_issuer(&resource).await?
        } else {
            self.discovery_issuer_url
                .clone()
                .unwrap_or_else(|| self.issuer_url.clone())
        };
        debug!("Loading discovery document from {discovery_issuer}");
        let metadata = CoreProviderMetadata::discover_async(
            IssuerUrl::new(discovery_issuer.to_string())?,
            &self.http_client,
        )
        .await
        .with_context(|| format!("Loading issuer data from {discovery_issuer}"))?;
        self.validate_issuer(&metadata, &discovery_issuer)?;
        Ok(metadata)
    }

    fn validate_issuer(
        &self,
        metadata: &CoreProviderMetadata,
        discovery_issuer: &str,
    ) -> Result<()> {
        let discovered = normalize_issuer(metadata.issuer().url().as_str());
        let expected = normalize_issuer(discovery_issuer);
        if discovered != expected {
            return Err(anyhow!(
                "issuer mismatch: expected {expected}, discovered {discovered}"
            ));
        }
        Ok(())
    }

    fn webfinger_host_and_scheme(resource: &str) -> Result<(String, String)> {
        if let Some(acct) = resource.strip_prefix("acct:") {
            let host = acct
                .split('@')
                .nth(1)
                .filter(|h| !h.is_empty())
                .ok_or_else(|| anyhow!("Invalid acct WebFinger resource: {resource}"))?;
            return Ok(("https".to_string(), host.to_string()));
        }
        let resource_url = Url::parse(resource).context("Invalid WebFinger resource URL")?;
        let host = resource_url
            .host_str()
            .ok_or_else(|| anyhow!("WebFinger resource URL has no host"))?
            .to_string();
        Ok((resource_url.scheme().to_string(), host))
    }

    fn webfinger_endpoint_url(resource: &str, issuer_url: &str) -> Result<Url> {
        if resource.starts_with("acct:") {
            let issuer = Url::parse(issuer_url).context("Invalid configured issuer URL")?;
            let (_, host) = Self::webfinger_host_and_scheme(resource)?;
            let scheme = issuer.scheme();
            let port = issuer.port_or_known_default().unwrap_or(443);
            let port_suffix =
                if (scheme == "https" && port == 443) || (scheme == "http" && port == 80) {
                    String::new()
                } else {
                    format!(":{port}")
                };
            Url::parse(&format!(
                "{scheme}://{host}{port_suffix}/.well-known/webfinger"
            ))
            .context("Invalid WebFinger URL")
        } else {
            let mut resource_url =
                Url::parse(resource).context("Invalid WebFinger resource URL")?;
            resource_url.set_path("/.well-known/webfinger");
            resource_url.set_query(None);
            resource_url.set_fragment(None);
            Ok(resource_url)
        }
    }

    async fn webfinger_issuer(&self, resource: &str) -> Result<String> {
        let mut webfinger_url = Self::webfinger_endpoint_url(resource, &self.issuer_url)?;
        webfinger_url
            .query_pairs_mut()
            .append_pair("resource", resource)
            .append_pair("rel", "http://openid.net/specs/connect/1.0/issuer");

        trace!("WebFinger lookup: {webfinger_url}");
        let response = self
            .http_client
            .get(webfinger_url)
            .send()
            .await
            .context("WebFinger request failed")?
            .error_for_status()
            .context("WebFinger request returned error status")?
            .text()
            .await
            .context("WebFinger response body unreadable")?;
        let response: Value =
            serde_json::from_str(&response).context("WebFinger response was not JSON")?;

        let links = response
            .get("links")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("WebFinger response missing links"))?;

        for link in links {
            if link.get("rel").and_then(|v| v.as_str())
                == Some("http://openid.net/specs/connect/1.0/issuer")
            {
                return link
                    .get("href")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| anyhow!("WebFinger issuer link missing href"));
            }
        }

        Err(anyhow!("WebFinger response has no OpenID issuer link"))
    }

    async fn build_inner(&self, mut metadata: CoreProviderMetadata) -> Result<OidcInner> {
        if let Some(authorization_endpoint_url) = &self.auth_url_override {
            trace!("Setting authorization endpoint to {authorization_endpoint_url}");
            metadata = metadata.set_authorization_endpoint(
                AuthUrl::new(authorization_endpoint_url.clone()).with_context(|| {
                    format!("Authorization endpoint is not valid: {authorization_endpoint_url}")
                })?,
            );
        }
        if let Some(token_endpoint_url) = &self.token_endpoint_override {
            trace!("Setting token endpoint to {token_endpoint_url}");
            metadata = metadata.set_token_endpoint(Some(
                TokenUrl::new(token_endpoint_url.clone())
                    .with_context(|| format!("Token endpoint is not valid: {token_endpoint_url}"))?,
            ));
        }
        if let Some(jwks_uri_url) = &self.jwks_uri_override {
            trace!("Setting JWKS URI to {jwks_uri_url}");
            metadata = metadata.set_jwks_uri(
                JsonWebKeySetUrl::new(jwks_uri_url.clone())
                    .with_context(|| format!("JWKS URI is not valid: {jwks_uri_url}"))?,
            );
        }
        trace!("Setting issuer for ID token validation to {}", self.issuer_url);
        metadata = metadata.set_issuer(
            IssuerUrl::new(self.issuer_url.clone())
                .context("Configured issuer URL is not valid")?,
        );

        let auth_type = resolve_auth_type(&metadata);
        let client = CoreClient::from_provider_metadata(
            metadata.clone(),
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
        )
        .set_auth_type(auth_type);

        Ok(OidcInner { client, metadata })
    }

    fn id_token_verifier(
        &self,
        inner: &OidcInner,
        jwks: CoreJsonWebKeySet,
    ) -> CoreIdTokenVerifier<'_> {
        CoreIdTokenVerifier::new_confidential_client(
            ClientId::new(self.client_id.clone()),
            ClientSecret::new(self.client_secret.clone()),
            inner.metadata.issuer().clone(),
            jwks,
        )
    }

    async fn fetch_jwks(&self, metadata: &CoreProviderMetadata) -> Result<CoreJsonWebKeySet> {
        CoreJsonWebKeySet::fetch_async(metadata.jwks_uri(), &self.http_client)
            .await
            .context("Failed to fetch JWKS")
    }

    async fn verify_id_token(
        &self,
        inner: &OidcInner,
        id_token: &CoreIdToken,
        nonce: &Nonce,
    ) -> Result<CoreIdTokenClaims> {
        for attempt in 0..2 {
            let jwks = self.fetch_jwks(&inner.metadata).await?;
            let verifier = self.id_token_verifier(inner, jwks);
            match id_token.claims(&verifier, nonce) {
                Ok(claims) => return Ok(claims.clone()),
                Err(
                    err @ ClaimsVerificationError::SignatureVerification(
                        SignatureVerificationError::CryptoError(_)
                        | SignatureVerificationError::AmbiguousKeyId(_)
                        | SignatureVerificationError::NoMatchingKey,
                    ),
                ) if attempt == 0 => {
                    debug!("ID token signature verification failed, refetching JWKS: {err:?}");
                    continue;
                }
                Err(err) => return Err(err.into()),
            }
        }
        Err(anyhow!(
            "ID token signature verification failed after JWKS refresh"
        ))
    }

    async fn verify_refreshed_id_token(
        &self,
        inner: &OidcInner,
        id_token: &CoreIdToken,
    ) -> Result<CoreIdTokenClaims> {
        for attempt in 0..2 {
            let jwks = self.fetch_jwks(&inner.metadata).await?;
            let verifier = self.id_token_verifier(inner, jwks);
            match id_token.claims(&verifier, |_nonce: Option<&Nonce>| Ok(())) {
                Ok(claims) => return Ok(claims.clone()),
                Err(
                    err @ ClaimsVerificationError::SignatureVerification(
                        SignatureVerificationError::CryptoError(_)
                        | SignatureVerificationError::AmbiguousKeyId(_)
                        | SignatureVerificationError::NoMatchingKey,
                    ),
                ) if attempt == 0 => {
                    debug!(
                        "Refresh ID token signature verification failed, refetching JWKS: {err:?}"
                    );
                    continue;
                }
                Err(err) => return Err(err.into()),
            }
        }
        Err(anyhow!(
            "Refresh ID token signature verification failed after JWKS refresh"
        ))
    }

    async fn resolve_distributed_claims(
        &self,
        inner: &OidcInner,
        id_token: &CoreIdToken,
        nonce: &Nonce,
    ) -> Result<()> {
        let payload = decode_jwt_payload(id_token.to_string().as_str())?;
        let Some(sources) = payload.get("_claim_sources").and_then(|v| v.as_object()) else {
            return Ok(());
        };
        if sources.is_empty() {
            return Ok(());
        }

        let jwks = self.fetch_jwks(&inner.metadata).await?;
        let verifier = self.id_token_verifier(inner, jwks);

        for source in sources.values() {
            self.resolve_distributed_source(source, &verifier, nonce)
                .await?;
        }

        debug!(
            "Resolved distributed claims from {} source(s)",
            sources.len()
        );
        Ok(())
    }

    async fn fetch_and_verify_distributed_jwt(
        &self,
        url: &str,
        verifier: &CoreIdTokenVerifier<'_>,
        nonce: &Nonce,
    ) -> Result<()> {
        trace!("Fetching distributed claim JWT from {url}");
        let jwt_str = self
            .http_client
            .get(url)
            .send()
            .await
            .with_context(|| format!("Failed to fetch distributed claim from {url}"))?
            .error_for_status()
            .with_context(|| format!("Distributed claim endpoint returned error: {url}"))?
            .text()
            .await
            .with_context(|| format!("Failed to read distributed claim body from {url}"))?;
        self.verify_distributed_jwt_str(&jwt_str, verifier, nonce)
    }

    fn verify_distributed_jwt_str(
        &self,
        jwt_str: &str,
        verifier: &CoreIdTokenVerifier<'_>,
        _nonce: &Nonce,
    ) -> Result<()> {
        let distributed_token: CoreIdToken = jwt_str.trim().parse().with_context(|| {
            format!(
                "Distributed claim response is not a valid JWT: {}",
                jwt_str.chars().take(40).collect::<String>()
            )
        })?;
        let _ = distributed_token
            .claims(verifier, |_: Option<&Nonce>| Ok(()))
            .context("Distributed claim JWT verification failed")?;
        Ok(())
    }

    async fn resolve_distributed_source(
        &self,
        source: &Value,
        verifier: &CoreIdTokenVerifier<'_>,
        nonce: &Nonce,
    ) -> Result<()> {
        if let Some(jwt_value) = source.get("JWT").and_then(|v| v.as_str()) {
            if jwt_value.starts_with("http") {
                self.fetch_and_verify_distributed_jwt(jwt_value, verifier, nonce)
                    .await?;
            } else if jwt_value.contains('.') {
                self.verify_distributed_jwt_str(jwt_value, verifier, nonce)?;
            }
            return Ok(());
        }

        if let Some(endpoint) = source.get("endpoint").and_then(|v| v.as_str()) {
            let mut request = self.http_client.get(endpoint);
            if let Some(token) = source.get("access_token").and_then(|v| v.as_str()) {
                request =
                    request.header(axum::http::header::AUTHORIZATION, format!("Bearer {token}"));
            }
            let jwt_str = request
                .send()
                .await
                .with_context(|| format!("Failed to fetch distributed claim from {endpoint}"))?
                .error_for_status()
                .with_context(|| format!("Distributed claim endpoint returned error: {endpoint}"))?
                .text()
                .await
                .with_context(|| {
                    format!("Failed to read distributed claim body from {endpoint}")
                })?;
            self.verify_distributed_jwt_str(&jwt_str, verifier, nonce)?;
        }
        Ok(())
    }

    async fn resolve_distributed_claims_from_userinfo_payload(
        &self,
        inner: &OidcInner,
        payload: &Value,
        nonce: &Nonce,
    ) -> Result<()> {
        let Some(sources) = payload.get("_claim_sources").and_then(|v| v.as_object()) else {
            return Ok(());
        };
        if sources.is_empty() {
            return Ok(());
        }
        let jwks = self.fetch_jwks(&inner.metadata).await?;
        let verifier = self.id_token_verifier(inner, jwks);
        for source in sources.values() {
            self.resolve_distributed_source(source, &verifier, nonce)
                .await?;
        }
        debug!(
            "Resolved distributed claims from userinfo ({} source(s))",
            sources.len()
        );
        Ok(())
    }

    pub(crate) async fn authorize_data(
        &self,
        authorize_request: AuthorizeRequestData,
    ) -> Result<AuthorizeData> {
        let inner = self.ensure_inner().await?;
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut b = inner
            .client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                || CsrfToken::new(authorize_request.state),
                Nonce::new_random,
            )
            .add_scope(Scope::new(authorize_request.scope))
            .set_pkce_challenge(pkce_challenge)
            .set_redirect_uri(Cow::Owned(RedirectUrl::new(
                authorize_request.redirect_uri,
            )?));
        if let Some(prompt) = authorize_request.prompt
            && prompt == "none"
        {
            b = b.add_prompt(openidconnect::core::CoreAuthPrompt::None);
        }
        if let Some(ui_locale) = authorize_request.ui_locales {
            b = b.add_ui_locale(openidconnect::LanguageTag::new(ui_locale));
        }
        if let Some(kc_idp_hint) = authorize_request.kc_idp_hint {
            b = b.add_extra_param("kc_idp_hint", kc_idp_hint);
        }
        if config::oidc_authorize_response_mode().as_deref() == Some("form_post") {
            b = b.add_extra_param("response_mode", "form_post");
        }

        let (auth_url, csrf_token, nonce) = b.url();

        Ok(AuthorizeData::new(
            auth_url,
            csrf_token,
            nonce,
            pkce_verifier,
        ))
    }

    pub(crate) async fn exchange_code(
        &self,
        data: TokenExchangeData,
    ) -> Result<(SessionTokens, String)> {
        let inner = self.ensure_inner().await?;
        let token_response = inner
            .client
            .exchange_code(AuthorizationCode::new(data.code))?
            .set_redirect_uri(Cow::Owned(RedirectUrl::new(data.redirect_uri.to_string())?))
            .set_pkce_verifier(PkceCodeVerifier::new(data.pkce_verifier))
            .request_async(&self.http_client)
            .await
            .map_err(|e| {
                debug_token_request_error(&e);
                anyhow!("Token exchange failed")
            })?;

        let id_token = token_response
            .id_token()
            .ok_or_else(|| anyhow!("Server did not return an ID token"))?;
        let nonce = Nonce::new(data.nonce);
        let claims = self.verify_id_token(&inner, id_token, &nonce).await?;

        if let Some(expected_access_token_hash) = claims.access_token_hash() {
            let jwks = self.fetch_jwks(&inner.metadata).await?;
            let verifier = self.id_token_verifier(&inner, jwks);
            let actual_access_token_hash = AccessTokenHash::from_token(
                token_response.access_token(),
                id_token.signing_alg()?,
                id_token.signing_key(&verifier)?,
            )?;
            if actual_access_token_hash != *expected_access_token_hash {
                return Err(anyhow!("Invalid access token"));
            }
        }

        self.resolve_distributed_claims(&inner, id_token, &nonce)
            .await?;

        debug!("Login successful {:?}", claims.subject());

        if config::conformance_mode() {
            let userinfo_payload = self
                .fetch_userinfo(
                    &inner,
                    token_response.access_token(),
                    Some(claims.subject()),
                )
                .await?;
            if let Some(payload) = userinfo_payload.as_ref() {
                validate_userinfo_subject(payload, claims.subject())?;
                self.resolve_distributed_claims_from_userinfo_payload(&inner, payload, &nonce)
                    .await?;
            }
            if config::oidc_jwks_refresh_after_userinfo() {
                let _ = self.fetch_jwks(&inner.metadata).await;
            }
        }

        token_response_to_session_tokens(&token_response)
            .map(|tr| (tr, claims.subject().to_string()))
    }

    pub(crate) async fn refresh_token(
        &self,
        refresh_token: &str,
        expected_subject: Option<&str>,
    ) -> Result<SessionTokens> {
        let inner = self.ensure_inner().await?;
        let token_response = inner
            .client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))?
            .request_async(&self.http_client)
            .await
            .map_err(|tokenerror| {
                debug_token_request_error(&tokenerror);
                anyhow!("Token exchange failed")
            })?;

        if let Some(id_token) = token_response.id_token() {
            let claims = self.verify_refreshed_id_token(&inner, id_token).await?;

            if let Some(expected) = expected_subject
                && claims.subject().as_str() != expected
            {
                return Err(anyhow!("ID token subject mismatch"));
            }

            if let Some(expected_access_token_hash) = claims.access_token_hash() {
                let jwks = self.fetch_jwks(&inner.metadata).await?;
                let verifier = self.id_token_verifier(&inner, jwks);
                let actual_access_token_hash = AccessTokenHash::from_token(
                    token_response.access_token(),
                    id_token.signing_alg()?,
                    id_token.signing_key(&verifier)?,
                )?;
                if actual_access_token_hash != *expected_access_token_hash {
                    return Err(anyhow!("Invalid access token"));
                }
            }

            if config::conformance_mode() {
                let userinfo_payload = self
                    .fetch_userinfo(
                        &inner,
                        token_response.access_token(),
                        Some(claims.subject()),
                    )
                    .await?;
                if let Some(payload) = userinfo_payload.as_ref() {
                    validate_userinfo_subject(payload, claims.subject())?;
                }
            }
        }

        token_response_to_session_tokens(&token_response)
    }

    async fn fetch_userinfo(
        &self,
        inner: &OidcInner,
        access_token: &oauth2::AccessToken,
        _subject: Option<&openidconnect::SubjectIdentifier>,
    ) -> Result<Option<Value>> {
        let method = config::oidc_userinfo_method();
        debug!("Fetching userinfo using {method} method");
        match method.as_str() {
            "body" => self.fetch_userinfo_form_body(inner, access_token).await,
            _ => self.fetch_userinfo_bearer_header(inner, access_token).await,
        }
    }

    async fn fetch_userinfo_bearer_header(
        &self,
        inner: &OidcInner,
        access_token: &oauth2::AccessToken,
    ) -> Result<Option<Value>> {
        let userinfo_url = inner
            .metadata
            .userinfo_endpoint()
            .ok_or_else(|| anyhow!("Provider metadata has no userinfo endpoint"))?
            .url()
            .to_string();
        let response = self
            .http_client
            .get(userinfo_url)
            .header(
                axum::http::header::AUTHORIZATION,
                format!("Bearer {}", access_token.secret()),
            )
            .send()
            .await
            .context("Userinfo request failed")?
            .error_for_status()
            .context("Userinfo request returned error status")?
            .text()
            .await
            .context("Userinfo response body unreadable")?;
        let payload: Value =
            serde_json::from_str(&response).context("Userinfo response was not valid JSON")?;
        debug!("Userinfo request completed (Bearer header)");
        Ok(Some(payload))
    }

    async fn fetch_userinfo_form_body(
        &self,
        inner: &OidcInner,
        access_token: &oauth2::AccessToken,
    ) -> Result<Option<Value>> {
        let userinfo_url = inner
            .metadata
            .userinfo_endpoint()
            .ok_or_else(|| anyhow!("Provider metadata has no userinfo endpoint"))?
            .url()
            .to_string();
        let body = format!("access_token={}", access_token.secret());

        let response = self
            .http_client
            .post(userinfo_url)
            .header(
                axum::http::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(body)
            .send()
            .await
            .context("Userinfo POST request failed")?
            .error_for_status()
            .context("Userinfo POST returned error status")?
            .text()
            .await
            .context("Userinfo POST response body unreadable")?;
        let payload: Value =
            serde_json::from_str(&response).context("Userinfo POST response was not valid JSON")?;

        debug!("Userinfo request completed (access_token in body)");
        Ok(Some(payload))
    }
}

fn debug_token_request_error(err: &impl std::fmt::Debug) {
    debug!("Token request error: {err:?}");
}

fn resolve_auth_type(metadata: &CoreProviderMetadata) -> AuthType {
    if let Some(method) = config::oidc_token_endpoint_auth_method() {
        return match method.as_str() {
            "client_secret_post" => AuthType::RequestBody,
            _ => AuthType::BasicAuth,
        };
    }

    let supported = metadata
        .token_endpoint_auth_methods_supported()
        .map(|methods| methods.iter().collect::<Vec<_>>());

    if let Some(methods) = supported {
        use openidconnect::core::CoreClientAuthMethod;
        let has_basic = methods
            .iter()
            .any(|m| matches!(m, CoreClientAuthMethod::ClientSecretBasic));
        let has_post = methods
            .iter()
            .any(|m| matches!(m, CoreClientAuthMethod::ClientSecretPost));
        if has_basic && !has_post {
            return AuthType::BasicAuth;
        }
        if has_post && !has_basic {
            return AuthType::RequestBody;
        }
    }

    AuthType::BasicAuth
}
