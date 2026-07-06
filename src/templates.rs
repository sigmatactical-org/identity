use askama::Template;
use sigma_theme::copyright_years;

#[derive(Template)]
#[template(path = "exampleapp.html")]
struct ExampleAppTemplate {
    copyright_years: String,
}

#[derive(Template)]
#[template(path = "conformance.html")]
struct ConformanceTemplate {
    copyright_years: String,
}

#[derive(Template)]
#[template(path = "register_success.html")]
struct RegisterSuccessTemplate {
    copyright_years: String,
    return_url: String,
    auto_approved: bool,
}

#[derive(Template)]
#[template(path = "register_verified.html")]
struct RegisterVerifiedTemplate {
    copyright_years: String,
    return_url: String,
    activated: bool,
}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    copyright_years: String,
    return_url: String,
    email: String,
    username: String,
    first_name: String,
    last_name: String,
    error: String,
}

/// All profile field values shared by the read-only view and the edit form.
/// Empty strings represent unfilled fields.
#[derive(Debug, Default, Clone)]
pub struct ProfileFields {
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub street: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country: String,
    pub birthdate: String,
    pub company: String,
}

/// A single label/value pair for the read-only profile view.
struct ProfileViewRow {
    label: String,
    value: String,
}

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileViewTemplate {
    copyright_years: String,
    return_url: String,
    edit_url: String,
    logout_url: String,
    saved: bool,
    rows: Vec<ProfileViewRow>,
}

#[derive(Template)]
#[template(path = "profile_edit.html")]
struct ProfileEditTemplate {
    copyright_years: String,
    return_url: String,
    cancel_url: String,
    error: String,
    username: String,
    email: String,
    first_name: String,
    last_name: String,
    phone: String,
    street: String,
    city: String,
    region: String,
    postal_code: String,
    country: String,
    birthdate: String,
    company: String,
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_exampleapp_html() -> Result<String, askama::Error> {
    ExampleAppTemplate {
        copyright_years: copyright_years(),
    }
    .render()
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_conformance_html() -> Result<String, askama::Error> {
    ConformanceTemplate {
        copyright_years: copyright_years(),
    }
    .render()
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_register_html(
    return_url: &str,
    email: &str,
    username: &str,
    first_name: &str,
    last_name: &str,
    error: Option<&str>,
) -> Result<String, askama::Error> {
    RegisterTemplate {
        copyright_years: copyright_years(),
        return_url: return_url.to_string(),
        email: email.to_string(),
        username: username.to_string(),
        first_name: first_name.to_string(),
        last_name: last_name.to_string(),
        error: error.unwrap_or_default().to_string(),
    }
    .render()
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_register_success_html(
    return_url: &str,
    auto_approved: bool,
) -> Result<String, askama::Error> {
    RegisterSuccessTemplate {
        copyright_years: copyright_years(),
        return_url: return_url.to_string(),
        auto_approved,
    }
    .render()
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_register_verified_html(
    return_url: &str,
    activated: bool,
) -> Result<String, askama::Error> {
    RegisterVerifiedTemplate {
        copyright_years: copyright_years(),
        return_url: return_url.to_string(),
        activated,
    }
    .render()
}

fn profile_row(label: &str, value: &str) -> ProfileViewRow {
    ProfileViewRow {
        label: label.to_string(),
        value: value.to_string(),
    }
}

/// Render the read-only profile view listing every field ("unfilled" for
/// empties), with Edit, Sign out, and (optionally) return-to-app actions.
///
/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_profile_view_html(
    return_url: &str,
    edit_url: &str,
    logout_url: &str,
    fields: &ProfileFields,
    saved: bool,
) -> Result<String, askama::Error> {
    let rows = vec![
        profile_row("Username", &fields.username),
        profile_row("Email", &fields.email),
        profile_row("First name", &fields.first_name),
        profile_row("Last name", &fields.last_name),
        profile_row("Phone", &fields.phone),
        profile_row("Company", &fields.company),
        profile_row("Street address", &fields.street),
        profile_row("City", &fields.city),
        profile_row("State / region", &fields.region),
        profile_row("Postal code", &fields.postal_code),
        profile_row("Country", &fields.country),
        profile_row("Date of birth", &fields.birthdate),
    ];
    ProfileViewTemplate {
        copyright_years: copyright_years(),
        return_url: return_url.to_string(),
        edit_url: edit_url.to_string(),
        logout_url: logout_url.to_string(),
        saved,
        rows,
    }
    .render()
}

/// Render the editable profile form.
///
/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_profile_html(
    return_url: &str,
    cancel_url: &str,
    fields: &ProfileFields,
    error: Option<&str>,
) -> Result<String, askama::Error> {
    ProfileEditTemplate {
        copyright_years: copyright_years(),
        return_url: return_url.to_string(),
        cancel_url: cancel_url.to_string(),
        error: error.unwrap_or_default().to_string(),
        username: fields.username.clone(),
        email: fields.email.clone(),
        first_name: fields.first_name.clone(),
        last_name: fields.last_name.clone(),
        phone: fields.phone.clone(),
        street: fields.street.clone(),
        city: fields.city.clone(),
        region: fields.region.clone(),
        postal_code: fields.postal_code.clone(),
        country: fields.country.clone(),
        birthdate: fields.birthdate.clone(),
        company: fields.company.clone(),
    }
    .render()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exampleapp_extends_sigma_theme_base() {
        let html = render_exampleapp_html().expect("example app template");
        assert!(html.contains("sigma-dial-root"));
        assert!(html.contains("/static/css/sigma-dial.css"));
        assert!(html.contains("id=\"loginStatus\""));
        assert!(html.contains("class=\"container app-demo\""));
    }

    #[test]
    fn conformance_extends_sigma_theme_base() {
        let html = render_conformance_html().expect("conformance template");
        assert!(html.contains("sigma-dial-root"));
        assert!(html.contains("id=\"conformance-login\""));
    }

    #[test]
    fn register_extends_sigma_theme_base() {
        let html = render_register_html(
            "http://localhost:3000/exampleapp/",
            "user@example.com",
            "newuser",
            "Ada",
            "Lovelace",
            None,
        )
        .expect("register template");
        assert!(html.contains("sigma-dial-root"));
        assert!(html.contains("name=\"return_url\""));
        assert!(html.contains("value=\"user@example.com\""));
        assert!(html.contains("password-toggle.js"));
        assert!(html.contains("data-password-target=\"password\""));
        assert!(html.contains("password-icon--show"));
        assert!(html.contains("fill=\"currentColor\""));
    }

    #[test]
    fn register_success_prompts_for_email_verification() {
        let html = render_register_success_html("http://localhost:3000/exampleapp/", false)
            .expect("register success template");
        assert!(html.contains("Check your email"));
        assert!(html.contains("disabled until you verify"));
        assert!(html.contains("Return to site"));
        assert!(!html.contains("register-success.js"));
    }

    #[test]
    fn register_success_shows_ready_to_sign_in_when_auto_approved() {
        let html = render_register_success_html("http://localhost:3000/exampleapp/", true)
            .expect("register success template");
        assert!(html.contains("Account ready"));
        assert!(html.contains("You can sign in now"));
        assert!(!html.contains("Check your email"));
    }

    #[test]
    fn register_success_without_return_url() {
        let html = render_register_success_html("", false).expect("register success template");
        assert!(html.contains("Check your email"));
        assert!(html.contains("You can close this page"));
    }

    #[test]
    fn register_verified_shows_activation_state() {
        let html = render_register_verified_html("http://localhost:3000/exampleapp/", true)
            .expect("register verified template");
        assert!(html.contains("Account activated"));
        assert!(html.contains("Continue"));
    }

    #[test]
    fn profile_edit_extends_sigma_theme_base() {
        let fields = ProfileFields {
            username: "bob".into(),
            email: "bob@example.com".into(),
            first_name: "Bob".into(),
            last_name: "Example".into(),
            ..ProfileFields::default()
        };
        let html = render_profile_html(
            "http://localhost:3000/exampleapp/",
            "http://localhost:3000/profile",
            &fields,
            None,
        )
        .expect("profile edit template");
        assert!(html.contains("sigma-dial-root"));
        assert!(html.contains("Edit profile"));
        assert!(html.contains("name=\"return_url\""));
        assert!(html.contains("value=\"bob@example.com\""));
        assert!(html.contains("name=\"phone\""));
        assert!(html.contains("name=\"company\""));
    }

    #[test]
    fn profile_view_lists_fields_and_actions() {
        let fields = ProfileFields {
            username: "bob".into(),
            email: "bob@example.com".into(),
            first_name: "Bob".into(),
            ..ProfileFields::default()
        };
        let html = render_profile_view_html(
            "http://localhost:3000/exampleapp/",
            "http://localhost:3000/profile?edit=1",
            "http://localhost:3000/auth/logout",
            &fields,
            true,
        )
        .expect("profile view template");
        assert!(html.contains("sigma-dial-root"));
        assert!(html.contains("bob@example.com"));
        // Empty fields render as "Unfilled".
        assert!(html.contains("Unfilled"));
        assert!(html.contains("Sign out"));
        assert!(html.contains("Edit"));
    }
}
