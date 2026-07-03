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
    redirect_url: String,
    redirect_delay_secs: u32,
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
    redirect_url: &str,
    redirect_delay_secs: u32,
) -> Result<String, askama::Error> {
    RegisterSuccessTemplate {
        copyright_years: copyright_years(),
        redirect_url: redirect_url.to_string(),
        redirect_delay_secs,
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
    fn register_success_includes_redirect_script() {
        let html = render_register_success_html(
            "http://localhost:3000/exampleapp/?registered=1",
            3,
        )
        .expect("register success template");
        assert!(html.contains("Account created"));
        assert!(html.contains("register-success.js"));
        assert!(html.contains("data-redirect-url=\"http://localhost:3000/exampleapp/?registered=1\""));
        assert!(html.contains("Continue now"));
    }

    #[test]
    fn register_success_without_redirect_url() {
        let html = render_register_success_html("", 3).expect("register success template");
        assert!(html.contains("Account created"));
        assert!(!html.contains("register-success.js"));
        assert!(html.contains("You can close this page"));
    }
}
