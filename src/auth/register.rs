use axum::{
    Extension, Form,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_macros::debug_handler;
use serde::Deserialize;
use tracing::{debug, error};
use url::Url;

use crate::config;
use crate::human_check;
use crate::templates;

use super::{
    allowlist::UriAllowlist,
    keycloak_admin::KeycloakAdmin,
    registration_adapter::{RegistrationAdapter, RegistrationContext, RegistrationOutcome},
};

#[derive(Clone)]
pub struct RegistrationDeps {
    pub settings: RegistrationAppSettings,
    pub admin: KeycloakAdmin,
    pub adapter: RegistrationAdapter,
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
    #[serde(default)]
    approved: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegisterVerifiedQuery {
    #[serde(default)]
    return_url: Option<String>,
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
    #[serde(default)]
    altcha: String,
}

#[debug_handler]
pub(crate) async fn register_form(
    State(settings): State<RegistrationAppSettings>,
    Extension(human_check): Extension<sigma_human_check::HumanCheck>,
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
        human_check,
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

    render_register_success_page(&params.return_url, params.approved.unwrap_or(false))
        .map_err(|error| *error)
}

#[debug_handler]
pub(crate) async fn register_verified(
    State(settings): State<RegistrationAppSettings>,
    Extension(keycloak): Extension<KeycloakAdmin>,
    Path(user_id): Path<String>,
    Query(params): Query<RegisterVerifiedQuery>,
) -> Result<Html<String>, Response> {
    if user_id.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Missing user").into_response());
    }

    let return_url = match params.return_url.filter(|value| !value.trim().is_empty()) {
        Some(return_url) => return_url,
        None => keycloak
            .registration_return_url(user_id.trim())
            .await
            .map_err(|error| {
                error!("Failed to load registration return URL: {:?}", error);
                (StatusCode::INTERNAL_SERVER_ERROR, "Activation failed").into_response()
            })?,
    };

    if !settings.is_return_url_allowed(&return_url) {
        debug!("return_url {} is not allowed", return_url);
        return Err((StatusCode::BAD_REQUEST, "Invalid return_url").into_response());
    }

    let activated = keycloak
        .activate_verified_user(user_id.trim())
        .await
        .map_err(|error| {
            error!("Failed to activate verified user: {:?}", error);
            (StatusCode::INTERNAL_SERVER_ERROR, "Activation failed").into_response()
        })?;

    render_register_verified_page(&return_url, activated).map_err(|error| *error)
}

#[debug_handler]
pub(crate) async fn register_submit(
    State(settings): State<RegistrationAppSettings>,
    Extension(keycloak): Extension<KeycloakAdmin>,
    Extension(adapter): Extension<RegistrationAdapter>,
    Extension(human_check): Extension<sigma_human_check::HumanCheck>,
    Form(form): Form<RegisterForm>,
) -> Result<Response, Response> {
    if !settings.is_return_url_allowed(&form.return_url) {
        return Err((StatusCode::BAD_REQUEST, "Invalid return_url").into_response());
    }

    if let Err(err) = human_check::verify_field(&human_check, &form.altcha) {
        return render_register_page(RegisterPage {
            return_url: form.return_url,
            email: form.email,
            username: form.username,
            first_name: form.first_name,
            last_name: form.last_name,
            error: Some(human_check::rejection_message(&err)),
            human_check,
        })
        .map(|html| html.into_response())
        .map_err(|error| *error);
    }

    if let Some(error) = validate_form(&form) {
        return render_register_page(RegisterPage {
            return_url: form.return_url,
            email: form.email,
            username: form.username,
            first_name: form.first_name,
            last_name: form.last_name,
            error: Some(error),
            human_check,
        })
        .map(|html| html.into_response())
        .map_err(|error| *error);
    }

    let username = form.username.trim();
    let email = form.email.trim();

    match keycloak
        .create_user(
            username,
            email,
            form.first_name.trim(),
            form.last_name.trim(),
            &form.password,
            &form.return_url,
        )
        .await
    {
        Ok(created) => {
            let context = RegistrationContext {
                verification_redirect_uri: verification_redirect_uri(&created.id),
            };
            match adapter
                .finalize_registration(&keycloak, &created, &context)
                .await
            {
                Ok(RegistrationOutcome::Approved) => Ok(Redirect::to(&register_success_location(
                    &form.return_url,
                    true,
                ))
                .into_response()),
                Ok(RegistrationOutcome::PendingEmailVerification) => Ok(Redirect::to(
                    &register_success_location(&form.return_url, false),
                )
                .into_response()),
                Err(finalize_error) => {
                    error!(
                        "Registration finalization failed after user create: {:?}",
                        finalize_error
                    );
                    if let Err(delete_error) = keycloak.delete_user(&created.id).await {
                        error!(
                            "Failed to roll back user after registration finalization failure: {:?}",
                            delete_error
                        );
                    }
                    render_register_page(RegisterPage {
                        return_url: form.return_url,
                        email: form.email,
                        username: form.username,
                        first_name: form.first_name,
                        last_name: form.last_name,
                        error: Some(finalization_error_message(&finalize_error)),
                        human_check: human_check.clone(),
                    })
                    .map(|html| html.into_response())
                    .map_err(|error| *error)
                }
            }
        }
        Err(error) => render_register_page(RegisterPage {
            return_url: form.return_url,
            email: form.email,
            username: form.username,
            first_name: form.first_name,
            last_name: form.last_name,
            error: Some(error.to_string()),
            human_check,
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
    human_check: sigma_human_check::HumanCheck,
}

fn render_register_success_page(
    return_url: &str,
    auto_approved: bool,
) -> Result<Html<String>, Box<Response>> {
    templates::render_register_success_html(return_url, auto_approved)
        .map(Html)
        .map_err(|error| {
            tracing::error!("Failed to render registration success page: {error}");
            Box::new((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response())
        })
}

fn render_register_verified_page(
    return_url: &str,
    activated: bool,
) -> Result<Html<String>, Box<Response>> {
    templates::render_register_verified_html(return_url, activated)
        .map(Html)
        .map_err(|error| {
            tracing::error!("Failed to render registration verified page: {error}");
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
        &page.human_check,
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

pub(crate) fn verification_redirect_uri(user_id: &str) -> String {
    format!(
        "{}/register/verified/{}",
        config::public_base_url().trim_end_matches('/'),
        user_id
    )
}

fn register_success_location(return_url: &str, approved: bool) -> String {
    let mut url = Url::parse("http://_/register/success").expect("valid success path");
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("return_url", return_url);
        if approved {
            pairs.append_pair("approved", "true");
        }
    }
    match url.query() {
        Some(query) => format!("/register/success?{query}"),
        None => "/register/success".to_string(),
    }
}

fn finalization_error_message(error: &anyhow::Error) -> String {
    let message = error.to_string();
    if message.contains("production account registration is not implemented") {
        return "Account registration is not available yet.".into();
    }
    "Your account could not be created. Please try again later.".into()
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
    fn register_success_location_encodes_return_url() {
        let location = register_success_location("http://store.example/?next=/", false);
        assert!(location.starts_with("/register/success?"));
        assert!(location.contains("return_url="));
        assert!(location.contains("store.example"));
        assert!(!location.contains("approved="));
    }

    #[test]
    fn register_success_location_includes_approved_when_auto_approved() {
        let location = register_success_location("http://store.example/", true);
        assert!(location.contains("approved=true"));
    }

    #[test]
    fn finalization_error_maps_prod_not_implemented() {
        let err = anyhow::anyhow!("production account registration is not implemented");
        assert_eq!(
            finalization_error_message(&err),
            "Account registration is not available yet."
        );
    }

    #[test]
    fn verification_redirect_uses_path_for_user_id() {
        let url = verification_redirect_uri("user-123");
        assert_eq!(url, "http://127.0.0.1:3000/register/verified/user-123");
        assert!(!url.contains('?'));
    }
}
