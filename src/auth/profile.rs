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

use super::{KeycloakAdmin, RegistrationAppSettings, SessionTokens};

#[derive(Clone)]
pub(crate) struct ProfileDeps {
    pub settings: RegistrationAppSettings,
    pub admin: KeycloakAdmin,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProfileQueryParams {
    #[serde(default)]
    return_url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProfileForm {
    return_url: String,
    username: String,
    email: String,
    first_name: String,
    last_name: String,
}

struct ProfilePage {
    return_url: String,
    username: String,
    email: String,
    first_name: String,
    last_name: String,
    error: Option<String>,
    saved: bool,
}

async fn session_tokens(session: &Session) -> Option<SessionTokens> {
    session.get(SESSION_KEY_JWT).await.ok().flatten()
}

fn profile_page_url(return_url: &str) -> String {
    let mut url = Url::parse(&format!(
        "{}/profile",
        config::public_base_url().trim_end_matches('/')
    ))
    .expect("valid profile base");
    if !return_url.is_empty() {
        url.query_pairs_mut().append_pair("return_url", return_url);
    }
    url.to_string()
}

fn login_redirect(return_url: &str) -> Redirect {
    let profile_url = profile_page_url(return_url);
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

fn render_profile_page(page: ProfilePage) -> Result<Html<String>, Box<Response>> {
    templates::render_profile_html(
        &page.return_url,
        &page.username,
        &page.email,
        &page.first_name,
        &page.last_name,
        page.error.as_deref(),
        page.saved,
    )
    .map(Html)
    .map_err(|error| {
        error!("Failed to render profile page: {error}");
        Box::new((StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response())
    })
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

    render_profile_page(ProfilePage {
        return_url: params.return_url,
        username: user.username.unwrap_or_default(),
        email: user.email.unwrap_or_default(),
        first_name: user.first_name.unwrap_or_default(),
        last_name: user.last_name.unwrap_or_default(),
        error: None,
        saved: false,
    })
    .map_err(|error| *error)
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
        return render_profile_page(ProfilePage {
            return_url: form.return_url,
            username: form.username,
            email: form.email,
            first_name: form.first_name,
            last_name: form.last_name,
            error: Some(message),
            saved: false,
        })
        .map(|html| html.into_response())
        .map_err(|error| *error);
    }

    admin
        .update_user_profile(
            &user_id,
            form.username.trim(),
            form.email.trim(),
            form.first_name.trim(),
            form.last_name.trim(),
        )
        .await
        .map_err(|error| {
            error!("Failed to update profile for {user_id}: {error:?}");
            render_profile_page(ProfilePage {
                return_url: form.return_url.clone(),
                username: form.username.clone(),
                email: form.email.clone(),
                first_name: form.first_name.clone(),
                last_name: form.last_name.clone(),
                error: Some("Unable to save profile changes. Try again.".into()),
                saved: false,
            })
            .map(|html| html.into_response())
            .map_err(|render_error| *render_error)
            .unwrap_err()
        })?;

    if !form.return_url.is_empty() {
        return Ok(Redirect::to(form.return_url.as_str()).into_response());
    }

    render_profile_page(ProfilePage {
        return_url: form.return_url,
        username: form.username,
        email: form.email,
        first_name: form.first_name,
        last_name: form.last_name,
        error: None,
        saved: true,
    })
    .map(|html| html.into_response())
    .map_err(|error| *error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_page_url_includes_return_url() {
        let url = profile_page_url("http://store.example/products/foo");
        assert!(url.contains("/profile?"));
        assert!(url.contains("return_url="));
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
