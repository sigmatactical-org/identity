use axum::{
    Extension,
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_macros::debug_handler;
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use tower_sessions::Session;
use tracing::error;
use url::Url;

use crate::config;
use crate::session::SESSION_KEY_JWT;
use crate::templates;

use super::{KeycloakAdmin, SessionTokens};

#[derive(Clone)]
pub(crate) struct AdminDeps {
    pub admin: KeycloakAdmin,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminListQuery {
    #[serde(default)]
    q: String,
    #[serde(default)]
    page: u32,
    #[serde(default)]
    notice: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AdminActionQuery {
    #[serde(default)]
    notice: String,
}

async fn session_tokens(session: &Session) -> Option<SessionTokens> {
    session.get(SESSION_KEY_JWT).await.ok().flatten()
}

fn admin_login_redirect() -> Redirect {
    let base = config::public_base_url();
    let public_base = base.trim_end_matches('/');
    let target = format!("{public_base}/admin/users");
    let callback = format!("{public_base}/auth/callback");
    let mut login_url = Url::parse(&format!("{public_base}/auth/login")).expect("valid login path");
    {
        let mut pairs = login_url.query_pairs_mut();
        pairs.append_pair("app_uri", &target);
        pairs.append_pair("redirect_uri", &callback);
        pairs.append_pair("scope", "openid");
    }
    Redirect::to(login_url.as_str())
}

fn tokens_are_admin(tokens: &SessionTokens) -> bool {
    let role_names = config::admin_realm_roles();
    let required_roles: Vec<&str> = role_names.iter().map(String::as_str).collect();
    !required_roles.is_empty() && tokens.has_any_realm_role(&required_roles)
}

/// Whether the current session belongs to an admin, without redirecting or
/// erroring when it doesn't (unlike [`require_admin`]) — for pages that only
/// conditionally show admin affordances.
pub(crate) async fn is_admin(session: &Session) -> bool {
    session_tokens(session)
        .await
        .is_some_and(|tokens| tokens_are_admin(&tokens))
}

async fn require_admin(session: &Session) -> Result<SessionTokens, Response> {
    let Some(tokens) = session_tokens(session).await else {
        return Err(admin_login_redirect().into_response());
    };
    if !tokens_are_admin(&tokens) {
        return Err((StatusCode::FORBIDDEN, "Administrator access required.").into_response());
    }
    Ok(tokens)
}

fn format_timestamp_ms(ms: Option<i64>) -> String {
    ms.and_then(|value| Utc.timestamp_millis_opt(value).single())
        .map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_default()
}

fn display_value(value: &str) -> String {
    if value.trim().is_empty() {
        "Unfilled".to_string()
    } else {
        value.to_string()
    }
}

fn notice_message(notice: &str) -> Option<&'static str> {
    match notice {
        "approved" => Some("Account approved."),
        "reset_sent" => Some("Password reset email sent."),
        "disabled" => Some("Account disabled."),
        "enabled" => Some("Account enabled."),
        "notified" => Some("Verification email sent."),
        _ => None,
    }
}

#[debug_handler]
pub(crate) async fn admin_user_list(
    Extension(keycloak): Extension<KeycloakAdmin>,
    session: Session,
    Query(query): Query<AdminListQuery>,
) -> Result<Html<String>, Response> {
    require_admin(&session).await?;
    let page = query.page;
    let page_size = 25_u32;
    let search = query.q.trim();
    let search_arg = if search.is_empty() {
        None
    } else {
        Some(search)
    };
    let users = keycloak
        .list_users(search_arg, page * page_size, page_size)
        .await
        .map_err(|error| {
            error!("Failed to list users: {error:?}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Unable to load users.").into_response()
        })?;
    let rows: Vec<templates::AdminUserRow> = users
        .into_iter()
        .map(|user| templates::AdminUserRow {
            id: user.id,
            username: user.username.unwrap_or_default(),
            email: user.email.unwrap_or_default(),
            name: format!(
                "{} {}",
                user.first_name.unwrap_or_default(),
                user.last_name.unwrap_or_default()
            )
            .trim()
            .to_string(),
            enabled: user.enabled,
            email_verified: user.email_verified,
            created: format_timestamp_ms(user.created_timestamp),
        })
        .collect();
    let html = templates::render_admin_users_html(
        search,
        page,
        page > 0,
        rows.len() == page_size as usize,
        rows,
        notice_message(&query.notice),
    )
    .map_err(|error| {
        error!("Failed to render admin user list: {error}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to render admin page.",
        )
            .into_response()
    })?;
    Ok(Html(html))
}

#[debug_handler]
pub(crate) async fn admin_user_detail(
    Extension(keycloak): Extension<KeycloakAdmin>,
    session: Session,
    Path(user_id): Path<String>,
    Query(query): Query<AdminActionQuery>,
) -> Result<Html<String>, Response> {
    require_admin(&session).await?;
    let profile = keycloak.get_user_profile(&user_id).await.map_err(|error| {
        error!("Failed to load user {user_id}: {error:?}");
        (StatusCode::NOT_FOUND, "User not found.").into_response()
    })?;
    let account = keycloak.get_user(&user_id).await.map_err(|error| {
        error!("Failed to load account {user_id}: {error:?}");
        (StatusCode::NOT_FOUND, "User not found.").into_response()
    })?;
    let created = account
        .created_timestamp
        .map(|ts| format_timestamp_ms(Some(ts)))
        .filter(|value| !value.is_empty())
        .or_else(|| attribute_value(&account.attributes, "createdDate"))
        .unwrap_or_default();
    let rows = vec![
        templates::ProfileViewRow {
            label: "User ID".into(),
            value: user_id.clone(),
        },
        templates::ProfileViewRow {
            label: "Username".into(),
            value: display_value(&profile.username.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "Email".into(),
            value: display_value(&profile.email.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "First name".into(),
            value: display_value(&profile.first_name.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "Last name".into(),
            value: display_value(&profile.last_name.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "Phone".into(),
            value: display_value(&profile.phone.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "Company".into(),
            value: display_value(&profile.company.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "Street address".into(),
            value: display_value(&profile.street.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "City".into(),
            value: display_value(&profile.city.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "State / region".into(),
            value: display_value(&profile.region.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "Postal code".into(),
            value: display_value(&profile.postal_code.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "Country".into(),
            value: display_value(&profile.country.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "Date of birth".into(),
            value: display_value(&profile.birthdate.clone().unwrap_or_default()),
        },
        templates::ProfileViewRow {
            label: "Created".into(),
            value: display_value(&created),
        },
        templates::ProfileViewRow {
            label: "Account enabled".into(),
            value: if account.enabled {
                "Yes".into()
            } else {
                "No".into()
            },
        },
        templates::ProfileViewRow {
            label: "Email verified".into(),
            value: if account.email_verified {
                "Yes".into()
            } else {
                "No".into()
            },
        },
    ];
    let html = templates::render_admin_user_html(
        &user_id,
        profile.username.unwrap_or_default(),
        account.enabled,
        account.email_verified,
        notice_message(&query.notice),
        rows,
    )
    .map_err(|error| {
        error!("Failed to render admin user detail: {error}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to render admin page.",
        )
            .into_response()
    })?;
    Ok(Html(html))
}

#[debug_handler]
pub(crate) async fn admin_user_approve(
    Extension(keycloak): Extension<KeycloakAdmin>,
    session: Session,
    Path(user_id): Path<String>,
) -> Result<Response, Response> {
    require_admin(&session).await?;
    keycloak.approve_user(&user_id).await.map_err(|error| {
        error!("Failed to approve user {user_id}: {error:?}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to approve account.",
        )
            .into_response()
    })?;
    Ok(Redirect::to(&format!("/admin/users/{user_id}?notice=approved")).into_response())
}

#[debug_handler]
pub(crate) async fn admin_user_reset_password(
    Extension(keycloak): Extension<KeycloakAdmin>,
    session: Session,
    Path(user_id): Path<String>,
) -> Result<Response, Response> {
    require_admin(&session).await?;
    keycloak
        .send_password_reset_email(&user_id)
        .await
        .map_err(|error| {
            error!("Failed to send password reset for {user_id}: {error:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to send password reset email.",
            )
                .into_response()
        })?;
    Ok(Redirect::to(&format!("/admin/users/{user_id}?notice=reset_sent")).into_response())
}

#[debug_handler]
pub(crate) async fn admin_user_disable(
    Extension(keycloak): Extension<KeycloakAdmin>,
    session: Session,
    Path(user_id): Path<String>,
) -> Result<Response, Response> {
    require_admin(&session).await?;
    keycloak
        .set_user_enabled(&user_id, false)
        .await
        .map_err(|error| {
            error!("Failed to disable user {user_id}: {error:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to disable account.",
            )
                .into_response()
        })?;
    Ok(Redirect::to(&format!("/admin/users/{user_id}?notice=disabled")).into_response())
}

#[debug_handler]
pub(crate) async fn admin_user_enable(
    Extension(keycloak): Extension<KeycloakAdmin>,
    session: Session,
    Path(user_id): Path<String>,
) -> Result<Response, Response> {
    require_admin(&session).await?;
    keycloak
        .set_user_enabled(&user_id, true)
        .await
        .map_err(|error| {
            error!("Failed to enable user {user_id}: {error:?}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Unable to enable account.",
            )
                .into_response()
        })?;
    Ok(Redirect::to(&format!("/admin/users/{user_id}?notice=enabled")).into_response())
}

#[debug_handler]
pub(crate) async fn admin_user_delete(
    Extension(keycloak): Extension<KeycloakAdmin>,
    session: Session,
    Path(user_id): Path<String>,
) -> Result<Response, Response> {
    require_admin(&session).await?;
    keycloak.delete_user(&user_id).await.map_err(|error| {
        error!("Failed to delete user {user_id}: {error:?}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to delete account.",
        )
            .into_response()
    })?;
    Ok(Redirect::to("/admin/users?notice=deleted").into_response())
}

#[debug_handler]
pub(crate) async fn admin_user_notify(
    Extension(keycloak): Extension<KeycloakAdmin>,
    session: Session,
    Path(user_id): Path<String>,
) -> Result<Response, Response> {
    require_admin(&session).await?;
    let account = keycloak.get_user(&user_id).await.map_err(|error| {
        error!("Failed to load account {user_id}: {error:?}");
        (StatusCode::NOT_FOUND, "User not found.").into_response()
    })?;
    let redirect_uri = super::register::verification_redirect_uri(&user_id);
    let result = if account.enabled {
        keycloak
            .send_verification_email(&user_id, &redirect_uri)
            .await
    } else {
        keycloak
            .send_verification_email_while_disabled(&user_id, &redirect_uri)
            .await
    };
    result.map_err(|error| {
        error!("Failed to send verification email for {user_id}: {error:?}");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Unable to send verification email.",
        )
            .into_response()
    })?;
    Ok(Redirect::to(&format!("/admin/users/{user_id}?notice=notified")).into_response())
}

fn attribute_value(
    attributes: &Option<std::collections::HashMap<String, Vec<String>>>,
    key: &str,
) -> Option<String> {
    attributes
        .as_ref()
        .and_then(|map| map.get(key))
        .and_then(|values| {
            values
                .iter()
                .find(|value| !value.trim().is_empty())
                .cloned()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notice_messages_are_mapped() {
        assert_eq!(notice_message("approved"), Some("Account approved."));
        assert_eq!(
            notice_message("reset_sent"),
            Some("Password reset email sent.")
        );
        assert_eq!(notice_message("disabled"), Some("Account disabled."));
        assert_eq!(notice_message("enabled"), Some("Account enabled."));
        assert_eq!(notice_message("notified"), Some("Verification email sent."));
        assert_eq!(notice_message(""), None);
    }
}
