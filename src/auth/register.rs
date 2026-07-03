use axum::{
    Extension, Form,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_macros::debug_handler;
use serde::Deserialize;
use tracing::debug;
use url::Url;

use crate::templates;

use super::{allowlist::UriAllowlist, keycloak_admin::KeycloakAdmin};

/// Seconds to wait on the success page before redirecting to `return_url`.
const SUCCESS_REDIRECT_DELAY_SECS: u32 = 3;

#[derive(Clone)]
pub struct RegistrationDeps {
    pub settings: RegistrationAppSettings,
    pub admin: KeycloakAdmin,
}

#[derive(Clone, Debug)]
pub struct RegistrationAppSettings {
    return_uris: UriAllowlist,
}

impl RegistrationAppSettings {
    pub fn new(return_uri_entries: Vec<String>) -> Self {
        Self {
            return_uris: UriAllowlist::new(return_uri_entries),
        }
    }

    pub(crate) fn is_return_url_allowed(&self, return_url: &str) -> bool {
        self.return_uris.is_allowed(return_url)
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterSuccessQuery {
    return_url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterQueryParams {
    return_url: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default, alias = "given_name")]
    first_name: Option<String>,
    #[serde(default, alias = "family_name")]
    last_name: Option<String>,
    #[serde(default)]
    username: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterForm {
    return_url: String,
    email: String,
    username: String,
    first_name: String,
    last_name: String,
    password: String,
    password_confirm: String,
}

#[debug_handler]
pub(crate) async fn register_form(
    State(settings): State<RegistrationAppSettings>,
    Query(params): Query<RegisterQueryParams>,
) -> Result<Html<String>, Response> {
    if !settings.is_return_url_allowed(&params.return_url) {
        debug!("return_url {} is not allowed", params.return_url);
        return Err((StatusCode::BAD_REQUEST, "Invalid return_url").into_response());
    }

    render_register_page(RegisterPage {
        return_url: params.return_url,
        email: params.email.unwrap_or_default(),
        username: params.username.unwrap_or_default(),
        first_name: params.first_name.unwrap_or_default(),
        last_name: params.last_name.unwrap_or_default(),
        error: None,
    })
    .map_err(|error| *error)
}

#[debug_handler]
pub(crate) async fn register_success(
    State(settings): State<RegistrationAppSettings>,
    Query(params): Query<RegisterSuccessQuery>,
) -> Result<Html<String>, Response> {
    if !params.return_url.is_empty() && !settings.is_return_url_allowed(&params.return_url) {
        debug!("return_url {} is not allowed", params.return_url);
        return Err((StatusCode::BAD_REQUEST, "Invalid return_url").into_response());
    }

    render_register_success_page(&params.return_url).map_err(|error| *error)
}

#[debug_handler]
pub(crate) async fn register_submit(
    State(settings): State<RegistrationAppSettings>,
    Extension(keycloak): Extension<KeycloakAdmin>,
    Form(form): Form<RegisterForm>,
) -> Result<Response, Response> {
    if !settings.is_return_url_allowed(&form.return_url) {
        return Err((StatusCode::BAD_REQUEST, "Invalid return_url").into_response());
    }

    if let Some(error) = validate_form(&form) {
        return render_register_page(RegisterPage {
            return_url: form.return_url,
            email: form.email,
            username: form.username,
            first_name: form.first_name,
            last_name: form.last_name,
            error: Some(error),
        })
        .map(|html| html.into_response())
        .map_err(|error| *error);
    }

    match keycloak
        .create_user(
            form.username.trim(),
            form.email.trim(),
            form.first_name.trim(),
            form.last_name.trim(),
            &form.password,
        )
        .await
    {
        Ok(()) => Ok(Redirect::to(&register_success_location(&form.return_url)).into_response()),
        Err(error) => render_register_page(RegisterPage {
            return_url: form.return_url,
            email: form.email,
            username: form.username,
            first_name: form.first_name,
            last_name: form.last_name,
            error: Some(error.to_string()),
        })
        .map(|html| html.into_response())
        .map_err(|error| *error),
    }
}

struct RegisterPage {
    return_url: String,
    email: String,
    username: String,
    first_name: String,
    last_name: String,
    error: Option<String>,
}

fn render_register_success_page(return_url: &str) -> Result<Html<String>, Box<Response>> {
    let redirect_url = if return_url.is_empty() {
        String::new()
    } else {
        success_redirect(return_url)
    };
    templates::render_register_success_html(&redirect_url, SUCCESS_REDIRECT_DELAY_SECS)
        .map(Html)
        .map_err(|error| {
            tracing::error!("Failed to render registration success page: {error}");
            Box::new((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response())
        })
}

fn render_register_page(page: RegisterPage) -> Result<Html<String>, Box<Response>> {
    templates::render_register_html(
        &page.return_url,
        &page.email,
        &page.username,
        &page.first_name,
        &page.last_name,
        page.error.as_deref(),
    )
    .map(Html)
    .map_err(|error| {
        tracing::error!("Failed to render registration page: {error}");
        Box::new((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response())
    })
}

fn validate_form(form: &RegisterForm) -> Option<String> {
    if form.username.trim().is_empty() {
        return Some("Username is required.".into());
    }
    if form.email.trim().is_empty() || !form.email.contains('@') {
        return Some("A valid email address is required.".into());
    }
    if form.password.len() < 8 {
        return Some("Password must be at least 8 characters.".into());
    }
    if form.password != form.password_confirm {
        return Some("Passwords do not match.".into());
    }
    None
}

fn register_success_location(return_url: &str) -> String {
    let mut url = Url::parse("http://_/register/success").expect("valid success path");
    url.query_pairs_mut().append_pair("return_url", return_url);
    match url.query() {
        Some(query) => format!("/register/success?{query}"),
        None => "/register/success".to_string(),
    }
}

fn success_redirect(return_url: &str) -> String {
    let Ok(mut url) = Url::parse(return_url) else {
        return return_url.to_string();
    };
    url.query_pairs_mut().append_pair("registered", "1");
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_url_allowlist() {
        let settings = RegistrationAppSettings::new(vec![
            "http://localhost:3000/*".to_string(),
            "https://app.example.com/welcome".to_string(),
        ]);
        assert!(settings.is_return_url_allowed("http://localhost:3000/done"));
        assert!(settings.is_return_url_allowed("https://app.example.com/welcome"));
        assert!(!settings.is_return_url_allowed("https://evil.example/"));
    }

    #[test]
    fn success_redirect_appends_query_param() {
        let url = success_redirect("http://localhost:3000/exampleapp/");
        assert!(url.contains("registered=1"));
    }

    #[test]
    fn register_success_location_encodes_return_url() {
        let location = register_success_location("http://store.example/?next=/");
        assert!(location.starts_with("/register/success?"));
        assert!(location.contains("return_url="));
        assert!(location.contains("store.example"));
    }
}
