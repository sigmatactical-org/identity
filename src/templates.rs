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

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfileTemplate {
    copyright_years: String,
    return_url: String,
    username: String,
    email: String,
    first_name: String,
    last_name: String,
    error: String,
    saved: bool,
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

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_profile_html(
    return_url: &str,
    username: &str,
    email: &str,
    first_name: &str,
    last_name: &str,
    error: Option<&str>,
    saved: bool,
) -> Result<String, askama::Error> {
    ProfileTemplate {
        copyright_years: copyright_years(),
        return_url: return_url.to_string(),
        username: username.to_string(),
        email: email.to_string(),
        first_name: first_name.to_string(),
        last_name: last_name.to_string(),
        error: error.unwrap_or_default().to_string(),
        saved,
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
        let html =
            render_register_verified_html("http://localhost:3000/exampleapp/", true)
                .expect("register verified template");
        assert!(html.contains("Account activated"));
        assert!(html.contains("Continue"));
    }

    #[test]
    fn profile_extends_sigma_theme_base() {
        let html = render_profile_html(
            "http://localhost:3000/exampleapp/",
            "bob",
            "bob@example.com",
            "Bob",
            "Example",
            None,
            false,
        )
        .expect("profile template");
        assert!(html.contains("sigma-dial-root"));
        assert!(html.contains("Edit profile"));
        assert!(html.contains("name=\"return_url\""));
        assert!(html.contains("value=\"bob@example.com\""));
    }
}
