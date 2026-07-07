use askama::Template;
use sigma_theme::copyright_years;
use sigma_theme::nav::{Breadcrumb, SiteHeader};

fn page_header(brand: &str) -> SiteHeader {
    SiteHeader::new(brand)
}

fn section_header(section: &str) -> SiteHeader {
    page_header("Sigma Identity")
        .with_breadcrumb(Breadcrumb::link("/", "Identity"))
        .with_breadcrumb(Breadcrumb::current(section))
}

fn admin_section_header(section: &str) -> SiteHeader {
    page_header("Sigma Identity")
        .with_breadcrumb(Breadcrumb::link("/", "Identity"))
        .with_breadcrumb(Breadcrumb::link("/admin", "Admin"))
        .with_breadcrumb(Breadcrumb::current(section))
}

fn site_nav(return_path: &str, show_contact_us: bool) -> Result<String, askama::Error> {
    crate::site_nav::render(return_path, show_contact_us, 0)
}

#[derive(Template)]
#[template(path = "exampleapp.html")]
struct ExampleAppTemplate {
    site_header: SiteHeader,
    site_nav: String,
    copyright_years: String,
}

#[derive(Template)]
#[template(path = "conformance.html")]
struct ConformanceTemplate {
    site_header: SiteHeader,
    site_nav: String,
    copyright_years: String,
}

#[derive(Template)]
#[template(path = "register_success.html")]
struct RegisterSuccessTemplate {
    site_header: SiteHeader,
    site_nav: String,
    copyright_years: String,
    return_url: String,
    auto_approved: bool,
}

#[derive(Template)]
#[template(path = "register_verified.html")]
struct RegisterVerifiedTemplate {
    site_header: SiteHeader,
    site_nav: String,
    copyright_years: String,
    return_url: String,
    activated: bool,
}

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    site_header: SiteHeader,
    site_nav: String,
    copyright_years: String,
    return_url: String,
    email: String,
    username: String,
    first_name: String,
    last_name: String,
    error: String,
    human_check_enabled: bool,
    human_check_challenge_url: String,
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

/// A single label/value pair for read-only profile and admin detail views.
pub(crate) struct ProfileViewRow {
    pub label: String,
    pub value: String,
}

/// A country `<option>` for the edit form's country dropdown.
struct CountryOption {
    name: String,
    selected: bool,
}

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileViewTemplate {
    site_header: SiteHeader,
    site_nav: String,
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
    site_header: SiteHeader,
    site_nav: String,
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
    postal_code: String,
    birthdate: String,
    company: String,
    countries: Vec<CountryOption>,
    region: String,
}

pub(crate) struct AdminUserRow {
    pub id: String,
    pub username: String,
    pub email: String,
    pub name: String,
    pub enabled: bool,
    pub email_verified: bool,
    pub created: String,
}

#[derive(Template)]
#[template(path = "admin_users.html")]
struct AdminUsersTemplate {
    site_header: SiteHeader,
    site_nav: String,
    copyright_years: String,
    search: String,
    has_prev: bool,
    has_next: bool,
    prev_page: u32,
    next_page: u32,
    users: Vec<AdminUserRow>,
}

#[derive(Template)]
#[template(path = "admin_user.html")]
struct AdminUserTemplate {
    site_header: SiteHeader,
    site_nav: String,
    copyright_years: String,
    user_id: String,
    username: String,
    enabled: bool,
    email_verified: bool,
    notice: String,
    rows: Vec<ProfileViewRow>,
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_exampleapp_html() -> Result<String, askama::Error> {
    ExampleAppTemplate {
        site_header: page_header("Sigma Identity"),
        site_nav: site_nav("/exampleapp/", true)?,
        copyright_years: copyright_years(),
    }
    .render()
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_conformance_html() -> Result<String, askama::Error> {
    ConformanceTemplate {
        site_header: page_header("Sigma Identity"),
        site_nav: site_nav("/conformance/", true)?,
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
    human_check: &sigma_human_check::HumanCheck,
) -> Result<String, askama::Error> {
    RegisterTemplate {
        site_header: section_header("Register"),
        site_nav: site_nav("/register", true)?,
        copyright_years: copyright_years(),
        return_url: return_url.to_string(),
        email: email.to_string(),
        username: username.to_string(),
        first_name: first_name.to_string(),
        last_name: last_name.to_string(),
        error: error.unwrap_or_default().to_string(),
        human_check_enabled: human_check.is_enabled(),
        human_check_challenge_url: "/human-check/challenge".to_string(),
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
        site_header: section_header("Registration complete"),
        site_nav: site_nav("/register/success", true)?,
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
        site_header: section_header("Email verified"),
        site_nav: site_nav("/register/verified", true)?,
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
        site_header: section_header("Profile"),
        site_nav: site_nav("/profile", true)?,
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
    let countries = crate::geo::COUNTRIES
        .iter()
        .map(|(_code, name)| CountryOption {
            name: (*name).to_string(),
            selected: name.eq_ignore_ascii_case(&fields.country),
        })
        .collect();
    ProfileEditTemplate {
        site_header: section_header("Edit profile"),
        site_nav: site_nav("/profile", true)?,
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
        postal_code: fields.postal_code.clone(),
        birthdate: fields.birthdate.clone(),
        company: fields.company.clone(),
        countries,
        region: fields.region.clone(),
    }
    .render()
}

/// Render the admin user list page.
///
/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_admin_users_html(
    search: &str,
    page: u32,
    has_prev: bool,
    has_next: bool,
    users: Vec<AdminUserRow>,
) -> Result<String, askama::Error> {
    AdminUsersTemplate {
        site_header: admin_section_header("Users"),
        site_nav: site_nav("/admin/users", true)?,
        copyright_years: copyright_years(),
        search: search.to_string(),
        has_prev,
        has_next,
        prev_page: page.saturating_sub(1),
        next_page: page + 1,
        users,
    }
    .render()
}

/// Render the admin user detail page.
///
/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_admin_user_html(
    user_id: &str,
    username: String,
    enabled: bool,
    email_verified: bool,
    notice: Option<&str>,
    rows: Vec<ProfileViewRow>,
) -> Result<String, askama::Error> {
    AdminUserTemplate {
        site_header: admin_section_header(&username),
        site_nav: site_nav(&format!("/admin/users/{user_id}"), true)?,
        copyright_years: copyright_years(),
        user_id: user_id.to_string(),
        username,
        enabled,
        email_verified,
        notice: notice.unwrap_or_default().to_string(),
        rows,
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
            &sigma_human_check::HumanCheck::disabled(),
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
        assert!(html.contains("id=\"address-country\""));
        assert!(html.contains("id=\"address-region-field\""));
        assert!(html.contains("address-select.js"));
        assert!(html.contains("data-initial-region"));
    }

    #[test]
    fn profile_edit_preserves_region_value_for_js() {
        let fields = ProfileFields {
            country: "United States".into(),
            region: "California".into(),
            ..ProfileFields::default()
        };
        let html = render_profile_html(
            "http://localhost:3000/exampleapp/",
            "http://localhost:3000/profile",
            &fields,
            None,
        )
        .expect("profile edit template");
        assert!(html.contains("data-initial-region=\"California\""));
        assert!(html.contains("value=\"United States\" selected"));
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
