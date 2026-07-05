use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use crate::config;

#[derive(Debug, Clone)]
pub(crate) struct CreatedUser {
    pub id: String,
}

#[derive(Clone)]
pub(crate) struct KeycloakAdmin {
    http_client: oauth2::reqwest::Client,
    server_base: String,
    realm: String,
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserPayload<'a> {
    username: &'a str,
    email: &'a str,
    first_name: &'a str,
    last_name: &'a str,
    enabled: bool,
    email_verified: bool,
    required_actions: [&'static str; 1],
    attributes: HashMap<&'static str, Vec<String>>,
    credentials: [PasswordCredential<'a>; 1],
}

#[derive(Debug, Serialize)]
struct PasswordCredential<'a> {
    #[serde(rename = "type")]
    credential_type: &'static str,
    value: &'a str,
    temporary: bool,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserRepresentation {
    username: Option<String>,
    email: Option<String>,
    first_name: Option<String>,
    last_name: Option<String>,
    enabled: bool,
    email_verified: bool,
    attributes: Option<HashMap<String, Vec<String>>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UserProfile {
    pub username: Option<String>,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateProfilePayload<'a> {
    username: &'a str,
    email: &'a str,
    first_name: &'a str,
    last_name: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateUserPayload {
    enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApproveUserPayload {
    enabled: bool,
    email_verified: bool,
    required_actions: Vec<&'static str>,
}

impl KeycloakAdmin {
    pub(crate) fn from_env() -> Result<Self> {
        let issuer_url = config::var("OIDC_ISSUER_URL")?;
        let client_id = config::var("OIDC_CLIENT_ID")?;
        let client_secret = config::var("OIDC_CLIENT_SECRET")?;
        let (mut server_base, realm) = parse_realm_issuer(&issuer_url)?;
        if let Some(admin_base) = config::keycloak_admin_base_url() {
            server_base = admin_base;
        }
        let danger_accept_invalid_certs = config::danger_accept_invalid_certs()?;
        let http_client = oauth2::reqwest::ClientBuilder::new()
            .danger_accept_invalid_certs(danger_accept_invalid_certs)
            .redirect(oauth2::reqwest::redirect::Policy::none())
            .build()
            .context("Failed to create Keycloak admin HTTP client")?;

        Ok(Self {
            http_client,
            server_base,
            realm,
            client_id,
            client_secret,
        })
    }

    pub(crate) async fn create_user(
        &self,
        username: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
        password: &str,
        return_url: &str,
    ) -> Result<CreatedUser> {
        let token = self.access_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users",
            self.server_base.trim_end_matches('/'),
            self.realm
        );
        let created_date = Utc::now().to_rfc3339();
        let mut attributes = HashMap::new();
        attributes.insert("createdDate", vec![created_date]);
        attributes.insert("registrationReturnUrl", vec![return_url.to_string()]);
        let payload = CreateUserPayload {
            username,
            email,
            first_name,
            last_name,
            enabled: false,
            email_verified: false,
            required_actions: ["VERIFY_EMAIL"],
            attributes,
            credentials: [PasswordCredential {
                credential_type: "password",
                value: password,
                temporary: false,
            }],
        };

        let body = serde_json::to_string(&payload).context("serialize create user payload")?;
        let response = self
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .bearer_auth(&token)
            .body(body)
            .send()
            .await
            .context("Keycloak create user request failed")?;

        match response.status().as_u16() {
            201 => {
                let location = response
                    .headers()
                    .get("Location")
                    .and_then(|value| value.to_str().ok())
                    .context("Keycloak create user response missing Location header")?;
                let user_id = user_id_from_location(location)?;
                Ok(CreatedUser { id: user_id })
            }
            409 => bail!("A user with that username or email already exists"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Keycloak create user failed with HTTP {status}: {body}")
            }
        }
    }

    pub(crate) async fn send_verification_email(
        &self,
        user_id: &str,
        redirect_uri: &str,
    ) -> Result<()> {
        let token = self.access_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}/execute-actions-email",
            self.server_base.trim_end_matches('/'),
            self.realm,
            user_id
        );
        let response = self
            .http_client
            .put(&url)
            .query(&[
                ("client_id", self.client_id.as_str()),
                ("redirect_uri", redirect_uri),
                ("lifespan", "86400"),
            ])
            .header("Content-Type", "application/json")
            .bearer_auth(&token)
            .body(r#"["VERIFY_EMAIL"]"#)
            .send()
            .await
            .context("Keycloak verification email request failed")?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Keycloak verification email failed with HTTP {status}: {body}")
    }

    /// Keycloak refuses to email disabled users; briefly enable, send, then disable again.
    pub(crate) async fn send_verification_email_while_disabled(
        &self,
        user_id: &str,
        redirect_uri: &str,
    ) -> Result<()> {
        self.set_user_enabled(user_id, true).await?;
        let result = self.send_verification_email(user_id, redirect_uri).await;
        if let Err(error) = self.set_user_enabled(user_id, false).await {
            tracing::warn!(
                "Failed to restore disabled state after verification email for {user_id}: {error:?}"
            );
        }
        result
    }

    pub(crate) async fn get_user(&self, user_id: &str) -> Result<UserRepresentation> {
        let token = self.access_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}",
            self.server_base.trim_end_matches('/'),
            self.realm,
            user_id
        );
        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Keycloak get user request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Keycloak get user failed with HTTP {status}: {body}");
        }

        let body = response
            .text()
            .await
            .context("Failed to read Keycloak user response")?;
        serde_json::from_str(&body).context("Failed to parse Keycloak user response")
    }

    pub(crate) async fn get_user_profile(&self, user_id: &str) -> Result<UserProfile> {
        let user = self.get_user(user_id).await?;
        Ok(UserProfile {
            username: user.username,
            email: user.email,
            first_name: user.first_name,
            last_name: user.last_name,
        })
    }

    pub(crate) async fn update_user_profile(
        &self,
        user_id: &str,
        username: &str,
        email: &str,
        first_name: &str,
        last_name: &str,
    ) -> Result<()> {
        let token = self.access_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}",
            self.server_base.trim_end_matches('/'),
            self.realm,
            user_id
        );
        let payload = UpdateProfilePayload {
            username,
            email,
            first_name,
            last_name,
        };
        let body = serde_json::to_string(&payload).context("serialize update profile payload")?;
        let response = self
            .http_client
            .put(&url)
            .header("Content-Type", "application/json")
            .bearer_auth(&token)
            .body(body)
            .send()
            .await
            .context("Keycloak update profile request failed")?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Keycloak update profile failed with HTTP {status}: {body}")
    }

    pub(crate) async fn registration_return_url(&self, user_id: &str) -> Result<String> {
        let user = self.get_user(user_id).await?;
        user.attributes
            .and_then(|attributes| attributes.get("registrationReturnUrl").cloned())
            .and_then(|values| values.into_iter().find(|value| !value.trim().is_empty()))
            .context("Registration return URL not found for user")
    }

    pub(crate) async fn approve_user(&self, user_id: &str) -> Result<()> {
        let token = self.access_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}",
            self.server_base.trim_end_matches('/'),
            self.realm,
            user_id
        );
        let payload = ApproveUserPayload {
            enabled: true,
            email_verified: true,
            required_actions: vec![],
        };
        let body = serde_json::to_string(&payload).context("serialize approve user payload")?;
        let response = self
            .http_client
            .put(&url)
            .header("Content-Type", "application/json")
            .bearer_auth(&token)
            .body(body)
            .send()
            .await
            .context("Keycloak approve user request failed")?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Keycloak approve user failed with HTTP {status}: {body}")
    }

    pub(crate) async fn set_user_enabled(&self, user_id: &str, enabled: bool) -> Result<()> {
        let token = self.access_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}",
            self.server_base.trim_end_matches('/'),
            self.realm,
            user_id
        );
        let payload = UpdateUserPayload { enabled };
        let body = serde_json::to_string(&payload).context("serialize update user payload")?;
        let response = self
            .http_client
            .put(&url)
            .header("Content-Type", "application/json")
            .bearer_auth(&token)
            .body(body)
            .send()
            .await
            .context("Keycloak update user request failed")?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Keycloak update user failed with HTTP {status}: {body}")
    }

    pub(crate) async fn delete_user(&self, user_id: &str) -> Result<()> {
        let token = self.access_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users/{}",
            self.server_base.trim_end_matches('/'),
            self.realm,
            user_id
        );
        let response = self
            .http_client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .context("Keycloak delete user request failed")?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Keycloak delete user failed with HTTP {status}: {body}")
    }

    pub(crate) async fn activate_verified_user(&self, user_id: &str) -> Result<bool> {
        let user = self.get_user(user_id).await?;
        if !user.email_verified {
            return Ok(false);
        }
        if !user.enabled {
            self.set_user_enabled(user_id, true).await?;
        }
        Ok(true)
    }

    async fn access_token(&self) -> Result<String> {
        let url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            self.server_base.trim_end_matches('/'),
            self.realm
        );
        let response = self
            .http_client
            .post(&url)
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
            ])
            .send()
            .await
            .context("Keycloak token request failed")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            bail!("Keycloak token request failed with HTTP {status}: {body}");
        }

        let body = response
            .text()
            .await
            .context("Failed to read Keycloak token response")?;
        let token: TokenResponse =
            serde_json::from_str(&body).context("Failed to parse Keycloak token response")?;
        Ok(token.access_token)
    }
}

fn user_id_from_location(location: &str) -> Result<String> {
    let user_id = location
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .context("Keycloak create user Location header has no user id")?;
    Ok(user_id.to_string())
}

fn parse_realm_issuer(issuer_url: &str) -> Result<(String, String)> {
    let issuer = Url::parse(issuer_url).context("Invalid OIDC issuer URL")?;
    let path = issuer.path().trim_end_matches('/');
    let realm = path
        .strip_prefix("/realms/")
        .filter(|name| !name.is_empty())
        .context("OIDC issuer URL must be of the form {scheme}://{host}/realms/{realm}")?;
    let server_base = issuer
        .join("/")
        .context("Invalid OIDC issuer origin")?
        .to_string()
        .trim_end_matches('/')
        .to_string();
    Ok((server_base, realm.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_issuer_extracts_realm_and_server() {
        let (base, realm) =
            parse_realm_issuer("http://127.0.0.1:8101/realms/multcorp").expect("parse issuer");
        assert_eq!(base, "http://127.0.0.1:8101");
        assert_eq!(realm, "multcorp");
    }

    #[test]
    fn location_header_yields_user_id() {
        assert_eq!(
            user_id_from_location("http://keycloak/admin/realms/multcorp/users/abc-123-def")
                .expect("user id"),
            "abc-123-def"
        );
    }
}
