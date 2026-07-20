mod admin_action_query;
mod admin_deps;
mod admin_list_query;
pub(crate) use admin_action_query::AdminActionQuery;
pub(crate) use admin_deps::AdminDeps;
pub(crate) use admin_list_query::AdminListQuery;

use axum::{
    Extension,
    extract::{Path, Query},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_macros::debug_handler;
use chrono::{TimeZone, Utc};
use tower_sessions::Session;
use tracing::error;

use crate::config;
use crate::templates;

use super::{KeycloakAdmin, SessionTokens, keycloak_admin::attribute_value, session_tokens};

fn admin_login_redirect() -> Redirect {
    let base = config::public_base_url();
    let target = format!("{}/admin/users", base.trim_end_matches('/'));
    Redirect::to(&super::login_url_for(&target))
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

/// A detail row, showing a placeholder for an unfilled value.
fn profile_row(label: &str, value: String) -> templates::ProfileViewRow {
    let value = if value.trim().is_empty() {
        "Unfilled".to_string()
    } else {
        value
    };
    templates::ProfileViewRow {
        label: label.to_string(),
        value,
    }
}

fn attribute_or_default(
    attributes: &Option<std::collections::HashMap<String, Vec<String>>>,
    key: &str,
) -> String {
    attribute_value(attributes, key).unwrap_or_default()
}

fn yes_no(value: bool) -> String {
    if value { "Yes" } else { "No" }.to_string()
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
    let service_account_rows = if page == 0 && search.is_empty() {
        keycloak
            .list_service_accounts()
            .await
            .map_err(|error| {
                error!("Failed to list service accounts: {error:?}");
            })
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let service_account_rows: Vec<templates::AdminServiceAccountRow> = service_account_rows
        .into_iter()
        .map(|row| templates::AdminServiceAccountRow {
            id: row.user.id,
            client_id: row.client_id,
            username: row.user.username.unwrap_or_default(),
            enabled: row.user.enabled,
            created: format_timestamp_ms(row.user.created_timestamp),
        })
        .collect();
    let html = templates::render_admin_users_html(
        search,
        page,
        page > 0,
        rows.len() == page_size as usize,
        rows,
        service_account_rows,
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
    // One fetch: derive both the profile fields and the account flags from the
    // same user representation (get_user_profile would re-request it).
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
    let username = account.username.unwrap_or_default();
    let enabled = account.enabled;
    let email_verified = account.email_verified;
    let attributes = account.attributes;
    let rows = vec![
        profile_row("User ID", user_id.clone()),
        profile_row("Username", username.clone()),
        profile_row("Email", account.email.unwrap_or_default()),
        profile_row("First name", account.first_name.unwrap_or_default()),
        profile_row("Last name", account.last_name.unwrap_or_default()),
        profile_row("Phone", attribute_or_default(&attributes, "phone")),
        profile_row("Company", attribute_or_default(&attributes, "company")),
        profile_row(
            "Street address",
            attribute_or_default(&attributes, "street"),
        ),
        profile_row("City", attribute_or_default(&attributes, "city")),
        profile_row(
            "State / region",
            attribute_or_default(&attributes, "region"),
        ),
        profile_row(
            "Postal code",
            attribute_or_default(&attributes, "postalCode"),
        ),
        profile_row("Country", attribute_or_default(&attributes, "country")),
        profile_row(
            "Date of birth",
            attribute_or_default(&attributes, "birthdate"),
        ),
        profile_row("Created", created),
        profile_row("Account enabled", yes_no(enabled)),
        profile_row("Email verified", yes_no(email_verified)),
    ];
    let html = templates::render_admin_user_html(
        &user_id,
        username,
        enabled,
        email_verified,
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
