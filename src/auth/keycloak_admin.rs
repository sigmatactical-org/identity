use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::config;

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

impl KeycloakAdmin {
    pub(crate) fn from_env() -> Result<Self> {
        let issuer_url = config::var("OIDC_ISSUER_URL")?;
        let client_id = config::var("OIDC_CLIENT_ID")?;
        let client_secret = config::var("OIDC_CLIENT_SECRET")?;
        let (server_base, realm) = parse_realm_issuer(&issuer_url)?;
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
    ) -> Result<()> {
        let token = self.access_token().await?;
        let url = format!(
            "{}/admin/realms/{}/users",
            self.server_base.trim_end_matches('/'),
            self.realm
        );
        let payload = CreateUserPayload {
            username,
            email,
            first_name,
            last_name,
            enabled: true,
            email_verified: false,
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
            201 => Ok(()),
            409 => bail!("A user with that username or email already exists"),
            status => {
                let body = response.text().await.unwrap_or_default();
                bail!("Keycloak create user failed with HTTP {status}: {body}")
            }
        }
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
}
