mod admin_service_account_row;
mod admin_user_row;
mod admin_user_template;
mod admin_users_template;
mod conformance_template;
mod country_option;
mod example_app_template;
mod index_template;
mod profile_edit_template;
mod profile_fields;
mod profile_view_row;
mod profile_view_template;
mod register_success_template;
mod register_template;
mod register_verified_template;
pub(crate) use admin_service_account_row::AdminServiceAccountRow;
pub(crate) use admin_user_row::AdminUserRow;
pub(crate) use admin_user_template::AdminUserTemplate;
pub(crate) use admin_users_template::AdminUsersTemplate;
pub(crate) use conformance_template::ConformanceTemplate;
pub(crate) use country_option::CountryOption;
pub(crate) use example_app_template::ExampleAppTemplate;
pub(crate) use index_template::IndexTemplate;
pub(crate) use profile_edit_template::ProfileEditTemplate;
pub use profile_fields::ProfileFields;
pub(crate) use profile_view_row::ProfileViewRow;
pub(crate) use profile_view_template::ProfileViewTemplate;
pub(crate) use register_success_template::RegisterSuccessTemplate;
pub(crate) use register_template::RegisterTemplate;
pub(crate) use register_verified_template::RegisterVerifiedTemplate;

use askama::Template;
use sigma_theme::copyright_years;
use sigma_theme::nav::{Breadcrumb, SiteHeader, site_menu};

fn page_header() -> SiteHeader {
    SiteHeader::new("Identity").with_menu(site_menu(None))
}

fn section_header(section: &str) -> SiteHeader {
    page_header()
        .with_breadcrumb(Breadcrumb::link("/", "Identity"))
        .with_breadcrumb(Breadcrumb::current(section))
}

fn admin_section_header(section: &str) -> SiteHeader {
    page_header()
        .with_breadcrumb(Breadcrumb::link("/", "Identity"))
        .with_breadcrumb(Breadcrumb::link("/admin", "Admin"))
        .with_breadcrumb(Breadcrumb::current(section))
}

fn site_nav(return_path: &str, show_contact_us: bool) -> Result<String, askama::Error> {
    crate::site_nav::render(return_path, show_contact_us, 0)
}

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_index_html(signed_in: bool, is_admin: bool) -> Result<String, askama::Error> {
    let base = crate::config::public_base_url();
    let links = sigma_identity_nav::auth_links(&base, &base, "/");
    IndexTemplate {
        site_header: page_header(),
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
        site_header: page_header(),
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
        site_header: page_header(),
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
        return_url,
        email,
        username,
        first_name,
        last_name,
        error: error.unwrap_or_default(),
        human_check_enabled: human_check.is_enabled(),
        human_check_challenge_url: "/human-check/challenge",
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
            name,
            selected: name.eq_ignore_ascii_case(&fields.country),
        })
        .collect();
    ProfileEditTemplate {
        site_header: section_header("Edit profile"),
        site_nav: site_nav("/profile", false)?,
        copyright_years: copyright_years(),
        return_url,
        cancel_url,
        error: error.unwrap_or_default(),
        username: &fields.username,
        email: &fields.email,
        first_name: &fields.first_name,
        last_name: &fields.last_name,
        phone: &fields.phone,
        street: &fields.street,
        city: &fields.city,
        postal_code: &fields.postal_code,
        birthdate: &fields.birthdate,
        company: &fields.company,
        countries,
        region: &fields.region,
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
        // Updates appears only in the global site menu, not the account cards.
        assert_eq!(html.matches("site-directory-card").count(), 3);
        assert!(html.contains("Sign out"));
        assert!(html.contains("Edit"));
        assert!(html.contains(">Back<"));
    }
}
