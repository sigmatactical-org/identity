use axum::{
    Extension, Form,
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_macros::debug_handler;
use serde::Deserialize;
use tower_sessions::Session;
use tracing::{debug, error};
use url::Url;

use crate::config;
use crate::session::SESSION_KEY_JWT;
use crate::templates;

use super::{KeycloakAdmin, ProfileInput, RegistrationAppSettings, SessionTokens};

#[derive(Clone)]
pub(crate) struct ProfileDeps {
    pub settings: RegistrationAppSettings,
    pub admin: KeycloakAdmin,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProfileQueryParams {
    #[serde(default)]
    return_url: String,
    #[serde(default)]
    edit: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProfileForm {
    return_url: String,
    username: String,
    email: String,
    first_name: String,
    last_name: String,
    #[serde(default)]
    phone: String,
    #[serde(default)]
    street: String,
    #[serde(default)]
    city: String,
    #[serde(default)]
    region: String,
    #[serde(default)]
    postal_code: String,
    #[serde(default)]
    country: String,
    #[serde(default)]
    birthdate: String,
    #[serde(default)]
    company: String,
}

async fn session_tokens(session: &Session) -> Option<SessionTokens> {
    session.get(SESSION_KEY_JWT).await.ok().flatten()
}

fn profile_url(return_url: &str, edit: bool) -> String {
    let mut url = Url::parse(&format!(
        "{}/profile",
        config::public_base_url().trim_end_matches('/')
    ))
    .expect("valid profile base");
    {
        let mut pairs = url.query_pairs_mut();
        if edit {
            pairs.append_pair("edit", "1");
        }
        if !return_url.is_empty() {
            pairs.append_pair("return_url", return_url);
        }
    }
    url.to_string()
}

/// Build the identity logout URL for the profile "Sign out" button. Returns the
/// user to `return_url` (or the identity home when absent) after sign-out.
fn logout_url(return_url: &str) -> String {
    let public_base = config::public_base_url();
    let identity_base = public_base.trim_end_matches('/');
    let app_uri = if return_url.is_empty() {
        format!("{identity_base}/")
    } else {
        return_url.to_string()
    };
    let mut url = Url::parse(&format!("{identity_base}/auth/logout")).expect("valid logout path");
    url.query_pairs_mut()
        .append_pair("app_uri", &app_uri)
        .append_pair(
            "redirect_uri",
            &format!("{identity_base}/auth/logoutcallback"),
        );
    url.to_string()
}

fn fields_from_form(form: &ProfileForm) -> templates::ProfileFields {
    templates::ProfileFields {
        username: form.username.clone(),
        email: form.email.clone(),
        first_name: form.first_name.clone(),
        last_name: form.last_name.clone(),
        phone: form.phone.clone(),
        street: form.street.clone(),
        city: form.city.clone(),
        region: form.region.clone(),
        postal_code: form.postal_code.clone(),
        country: form.country.clone(),
        birthdate: form.birthdate.clone(),
        company: form.company.clone(),
    }
}

fn input_from_form(form: &ProfileForm) -> ProfileInput {
    ProfileInput {
        username: form.username.trim().to_string(),
        email: form.email.trim().to_string(),
        first_name: form.first_name.trim().to_string(),
        last_name: form.last_name.trim().to_string(),
        phone: form.phone.trim().to_string(),
        street: form.street.trim().to_string(),
        city: form.city.trim().to_string(),
        region: form.region.trim().to_string(),
        postal_code: form.postal_code.trim().to_string(),
        country: form.country.trim().to_string(),
        birthdate: form.birthdate.trim().to_string(),
        company: form.company.trim().to_string(),
    }
}

fn render_view(
    return_url: &str,
    fields: &templates::ProfileFields,
    saved: bool,
) -> Result<Html<String>, Box<Response>> {
    let edit_url = profile_url(return_url, true);
    let logout = logout_url(return_url);
    templates::render_profile_view_html(return_url, &edit_url, &logout, fields, saved)
        .map(Html)
        .map_err(|error| {
            error!("Failed to render profile view: {error}");
            Box::new((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response())
        })
}

fn render_edit(
    return_url: &str,
    fields: &templates::ProfileFields,
    error_message: Option<&str>,
) -> Result<Html<String>, Box<Response>> {
    let cancel_url = profile_url(return_url, false);
    templates::render_profile_html(return_url, &cancel_url, fields, error_message)
        .map(Html)
        .map_err(|error| {
            error!("Failed to render profile edit: {error}");
            Box::new((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response())
        })
}

fn login_redirect(return_url: &str) -> Redirect {
    let profile_url = profile_url(return_url, false);
    let public_base = config::public_base_url();
    let identity_base = public_base.trim_end_matches('/');
    let callback = format!("{identity_base}/auth/callback");
    let mut login_url =
        Url::parse(&format!("{identity_base}/auth/login")).expect("valid login path");
    {
        let mut pairs = login_url.query_pairs_mut();
        pairs.append_pair("app_uri", &profile_url);
        pairs.append_pair("redirect_uri", &callback);
        pairs.append_pair("scope", "openid");
    }
    Redirect::to(login_url.as_str())
}

fn validate_form(form: &ProfileForm) -> Option<String> {
    if form.username.trim().is_empty() {
        return Some("Username is required.".into());
    }
    if form.email.trim().is_empty() || !form.email.contains('@') {
        return Some("A valid email address is required.".into());
    }
    None
}

#[debug_handler]
pub(crate) async fn profile_form(
    State(settings): State<RegistrationAppSettings>,
    Extension(admin): Extension<KeycloakAdmin>,
    session: Session,
    Query(params): Query<ProfileQueryParams>,
) -> Result<Html<String>, Response> {
    if !params.return_url.is_empty() && !settings.is_return_url_allowed(&params.return_url) {
        debug!("return_url {} is not allowed", params.return_url);
        return Err((StatusCode::BAD_REQUEST, "Invalid return_url").into_response());
    }
    let Some(tokens) = session_tokens(&session).await else {
        return Err(login_redirect(&params.return_url).into_response());
    };
    let Some(user_id) = tokens.subject() else {
        return Err((StatusCode::UNAUTHORIZED, "Invalid session").into_response());
    };

    let user = admin.get_user_profile(&user_id).await.map_err(|error| {
        error!("Failed to load profile for {user_id}: {error:?}");
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
    })?;

    let fields = templates::ProfileFields {
        username: user.username.unwrap_or_default(),
        email: user.email.unwrap_or_default(),
        first_name: user.first_name.unwrap_or_default(),
        last_name: user.last_name.unwrap_or_default(),
        phone: user.phone.unwrap_or_default(),
        street: user.street.unwrap_or_default(),
        city: user.city.unwrap_or_default(),
        region: user.region.unwrap_or_default(),
        postal_code: user.postal_code.unwrap_or_default(),
        country: user.country.unwrap_or_default(),
        birthdate: user.birthdate.unwrap_or_default(),
        company: user.company.unwrap_or_default(),
    };

    let editing = params
        .edit
        .as_deref()
        .is_some_and(|value| matches!(value, "1" | "true"));
    if editing {
        render_edit(&params.return_url, &fields, None).map_err(|error| *error)
    } else {
        render_view(&params.return_url, &fields, false).map_err(|error| *error)
    }
}

#[debug_handler]
pub(crate) async fn profile_submit(
    State(settings): State<RegistrationAppSettings>,
    Extension(admin): Extension<KeycloakAdmin>,
    session: Session,
    Form(form): Form<ProfileForm>,
) -> Result<Response, Response> {
    if !form.return_url.is_empty() && !settings.is_return_url_allowed(&form.return_url) {
        debug!("return_url {} is not allowed", form.return_url);
        return Err((StatusCode::BAD_REQUEST, "Invalid return_url").into_response());
    }
    let Some(tokens) = session_tokens(&session).await else {
        return Err(login_redirect(&form.return_url).into_response());
    };
    let Some(user_id) = tokens.subject() else {
        return Err((StatusCode::UNAUTHORIZED, "Invalid session").into_response());
    };

    if let Some(message) = validate_form(&form) {
        let fields = fields_from_form(&form);
        return render_edit(&form.return_url, &fields, Some(&message))
            .map(IntoResponse::into_response)
            .map_err(|error| *error);
    }

    if let Err(error) = admin
        .update_user_profile(&user_id, &input_from_form(&form))
        .await
    {
        error!("Failed to update profile for {user_id}: {error:?}");
        let fields = fields_from_form(&form);
        return render_edit(
            &form.return_url,
            &fields,
            Some("Unable to save profile changes. Try again."),
        )
        .map(IntoResponse::into_response)
        .map_err(|error| *error);
    }

    if !form.return_url.is_empty() {
        return Ok(Redirect::to(form.return_url.as_str()).into_response());
    }

    let fields = fields_from_form(&form);
    render_view(&form.return_url, &fields, true)
        .map(IntoResponse::into_response)
        .map_err(|error| *error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_url_includes_return_url() {
        let url = profile_url("http://store.example/products/foo", false);
        assert!(url.contains("/profile?"));
        assert!(url.contains("return_url="));
    }

    #[test]
    fn profile_edit_url_sets_edit_flag() {
        let url = profile_url("http://store.example/products/foo", true);
        assert!(url.contains("edit=1"));
        assert!(url.contains("return_url="));
    }

    #[test]
    fn logout_url_targets_identity_logout() {
        let url = logout_url("http://store.example/");
        assert!(url.contains("/auth/logout?"));
        assert!(url.contains("app_uri="));
        assert!(url.contains("redirect_uri="));
        assert!(url.contains("logoutcallback"));
    }

    #[test]
    fn login_redirect_targets_profile_after_sign_in() {
        let redirect = login_redirect("http://store.example/");
        let response = redirect.into_response();
        let location = response.headers().get("location").unwrap();
        let location = location.to_str().unwrap();
        assert!(location.contains("/auth/login?"));
        assert!(location.contains("app_uri="));
        assert!(location.contains("%2Fprofile"));
    }
}
