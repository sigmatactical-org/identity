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
#[template(path = "index.html")]
struct IndexTemplate {
    site_header: SiteHeader,
    site_nav: String,
    copyright_years: String,
    signed_in: bool,
    is_admin: bool,
    profile_url: String,
    register_url: String,
    sign_in_url: String,
    addresses_url: String,
    payments_url: String,
    orders_url: String,
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
    addresses_url: String,
    payments_url: String,
    orders_url: String,
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

/// A client's service-account user, for the admin "Service accounts" table.
pub(crate) struct AdminServiceAccountRow {
    pub id: String,
    pub client_id: String,
    pub username: String,
    pub enabled: bool,
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
    service_accounts: Vec<AdminServiceAccountRow>,
    notice: String,
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
pub fn render_index_html(signed_in: bool, is_admin: bool) -> Result<String, askama::Error> {
    let base = crate::config::public_base_url();
    let links = sigma_identity_nav::auth_links(&base, &base, "/");
    IndexTemplate {
        site_header: page_header("Sigma Identity"),
        site_nav: site_nav("/", false)?,
        copyright_years: copyright_years(),
        signed_in,
        is_admin,
        profile_url: links.profile_url,
        register_url: links.register_url,
        sign_in_url: links.sign_in_url,
        addresses_url: crate::config::addresses_public_base_url(),
        payments_url: crate::config::payments_public_base_url(),
        orders_url: crate::config::orders_public_base_url(),
    }
    .render()
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_exampleapp_html() -> Result<String, askama::Error> {
    ExampleAppTemplate {
        site_header: page_header("Sigma Identity"),
        site_nav: site_nav("/exampleapp/", false)?,
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
        site_nav: site_nav("/conformance/", false)?,
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
        site_nav: site_nav("/register", false)?,
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
        site_nav: site_nav("/register/success", false)?,
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
        site_nav: site_nav("/register/verified", false)?,
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

/// Render the read-only profile view listing only filled fields, with Edit,
/// Sign out, account-service links, and (optionally) a return-to-app Back
/// control.
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
    let rows = [
        ("Username", fields.username.as_str()),
        ("Email", fields.email.as_str()),
        ("First name", fields.first_name.as_str()),
        ("Last name", fields.last_name.as_str()),
        ("Phone", fields.phone.as_str()),
        ("Company", fields.company.as_str()),
        ("Street address", fields.street.as_str()),
        ("City", fields.city.as_str()),
        ("State / region", fields.region.as_str()),
        ("Postal code", fields.postal_code.as_str()),
        ("Country", fields.country.as_str()),
        ("Date of birth", fields.birthdate.as_str()),
    ]
    .into_iter()
    .filter(|(_, value)| !value.trim().is_empty())
    .map(|(label, value)| profile_row(label, value))
    .collect();
    ProfileViewTemplate {
        site_header: section_header("Profile"),
        site_nav: site_nav("/profile", false)?,
        copyright_years: copyright_years(),
        return_url: return_url.to_string(),
        edit_url: edit_url.to_string(),
        logout_url: logout_url.to_string(),
        addresses_url: crate::config::addresses_public_base_url(),
        payments_url: crate::config::payments_public_base_url(),
        orders_url: crate::config::orders_public_base_url(),
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
        site_nav: site_nav("/profile", false)?,
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
    service_accounts: Vec<AdminServiceAccountRow>,
    notice: Option<&str>,
) -> Result<String, askama::Error> {
    AdminUsersTemplate {
        site_header: admin_section_header("Users"),
        site_nav: site_nav("/admin/users", false)?,
        copyright_years: copyright_years(),
        search: search.to_string(),
        has_prev,
        has_next,
        prev_page: page.saturating_sub(1),
        next_page: page + 1,
        users,
        service_accounts,
        notice: notice.unwrap_or_default().to_string(),
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
        site_nav: site_nav(&format!("/admin/users/{user_id}"), false)?,
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
    fn index_hides_admin_section_for_anonymous_visitor() {
        let html = render_index_html(false, false).expect("index template");
        assert!(html.contains("sigma-dial-root"));
        assert!(!html.contains("href=\"/admin/users\""));
    }

    #[test]
    fn index_shows_users_link_for_admin() {
        let html = render_index_html(true, true).expect("index template");
        assert!(html.contains("href=\"/admin/users\""));
        assert!(html.contains(">Users<"));
    }

    #[test]
    fn index_shows_sign_in_and_register_when_signed_out() {
        let html = render_index_html(false, false).expect("index template");
        assert!(html.contains("/auth/login?"));
        assert!(html.contains("/register?return_url="));
        assert!(html.contains(">Sign in<"));
        assert!(html.contains(">Register<"));
        assert!(!html.contains(">Update profile<"));
        assert!(html.contains("site-directory-name\">Addresses</span>"));
        assert!(html.contains("site-directory-name\">Payments</span>"));
        assert!(html.contains("site-directory-name\">Orders</span>"));
        let sign_in_pos = html.find(">Sign in<").expect("sign in link");
        let register_pos = html.find(">Register<").expect("register link");
        assert!(
            sign_in_pos < register_pos,
            "Sign in should appear above Register"
        );
    }

    #[test]
    fn admin_user_shows_disable_and_delete_for_enabled_verified_account() {
        let html = render_admin_user_html("u1", "alice".to_string(), true, true, None, Vec::new())
            .expect("admin user template");
        assert!(html.contains("/admin/users/u1/disable"));
        assert!(html.contains("/admin/users/u1/delete"));
        assert!(!html.contains("/admin/users/u1/enable\""));
        assert!(!html.contains("/admin/users/u1/notify"));
        assert!(!html.contains("Approve account"));
    }

    #[test]
    fn admin_user_shows_enable_notify_and_approve_for_disabled_unverified_account() {
        let html =
            render_admin_user_html("u1", "alice".to_string(), false, false, None, Vec::new())
                .expect("admin user template");
        assert!(html.contains("/admin/users/u1/enable"));
        assert!(html.contains("/admin/users/u1/notify"));
        assert!(html.contains("Approve account"));
        assert!(!html.contains("/admin/users/u1/disable\""));
    }

    #[test]
    fn admin_users_list_shows_notice() {
        let html = render_admin_users_html(
            "",
            0,
            false,
            false,
            Vec::new(),
            Vec::new(),
            Some("Deleted."),
        )
        .expect("admin users template");
        assert!(html.contains("Deleted."));
    }

    #[test]
    fn admin_users_list_shows_service_accounts() {
        let html = render_admin_users_html(
            "",
            0,
            false,
            false,
            Vec::new(),
            vec![AdminServiceAccountRow {
                id: "sa-1".to_string(),
                client_id: "sigma-updates-ci".to_string(),
                username: "service-account-sigma-updates-ci".to_string(),
                enabled: true,
                created: "2026-01-01".to_string(),
            }],
            None,
        )
        .expect("admin users template");
        assert!(html.contains("sigma-updates-ci"));
        assert!(html.contains("/admin/users/sa-1"));
    }

    #[test]
    fn index_shows_update_profile_when_signed_in() {
        let html = render_index_html(true, false).expect("index template");
        assert!(html.contains("/profile?return_url="));
        assert!(html.contains(">Update profile<"));
        assert!(!html.contains(">Register / Sign in<"));
    }

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
        assert!(html.contains("Bob"));
        // Empty optional fields are omitted from the read-only view.
        assert!(!html.contains("Unfilled"));
        assert!(!html.contains("Phone"));
        assert!(html.contains("Addresses"));
        assert!(html.contains("Payments"));
        assert!(html.contains("Orders"));
        assert!(!html.contains("Updates"));
        assert!(html.contains("Sign out"));
        assert!(html.contains("Edit"));
        assert!(html.contains(">Back<"));
    }
}
