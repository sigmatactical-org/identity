//! Reusable Sigma sign-in / "Welcome" navbar widget shared across Sigma web
//! services (store, cart, ...).
//!
//! [`auth_links`] builds identity URLs for a given app and return path;
//! [`render_auth_nav`] renders the client-side auth widget consumed by
//! `sigma-theme`'s `store-nav-auth.js` (it keeps the same element IDs, so the
//! shared JS toggles signed-in/out state without changes). The widget shows a
//! "Sign in" button when signed out and a "Welcome, {username}" link to the
//! profile page when signed in; sign-out lives on the profile page.

use askama::Template;

/// Resolved identity URLs for a service's navbar.
pub struct AuthLinks {
    /// Identity login URL that returns the user to the calling app.
    pub sign_in_url: String,
    /// Identity profile page URL for the signed-in user.
    pub profile_url: String,
    /// Identity registration page URL that returns the user to the calling app.
    pub register_url: String,
    /// Identity service public base URL (trailing slash), used by the widget JS.
    pub identity_base_url: String,
}

/// Build the identity URLs that return the user to `return_path` on the calling
/// app.
///
/// - `identity_base`: public base URL of the identity service.
/// - `app_base`: public base URL of the calling app (used to build `app_uri`).
/// - `return_path`: path on the calling app to return to after sign-in.
#[must_use]
pub fn auth_links(identity_base: &str, app_base: &str, return_path: &str) -> AuthLinks {
    let app_uri = join_url(app_base, return_path);
    let identity_root = identity_base.trim_end_matches('/');
    let callback_uri = format!("{identity_root}/auth/callback");

    let sign_in_url = format!(
        "{identity_root}/auth/login?app_uri={app_uri}&redirect_uri={callback}&scope=openid",
        app_uri = percent_encode(&app_uri),
        callback = percent_encode(&callback_uri),
    );
    let profile_url = format!(
        "{identity_root}/profile?return_url={}",
        percent_encode(&app_uri)
    );
    let register_url = format!(
        "{identity_root}/register?return_url={}",
        percent_encode(&app_uri)
    );

    AuthLinks {
        sign_in_url,
        profile_url,
        register_url,
        identity_base_url: format!("{identity_root}/"),
    }
}

#[derive(Template)]
#[template(path = "auth_nav.html")]
struct AuthNavTemplate<'a> {
    sign_in_url: &'a str,
    identity_base_url: &'a str,
    profile_url: &'a str,
}

/// Render the sign-in / "Welcome" navbar widget HTML for embedding in a page.
///
/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_auth_nav(links: &AuthLinks) -> Result<String, askama::Error> {
    AuthNavTemplate {
        sign_in_url: &links.sign_in_url,
        identity_base_url: &links.identity_base_url,
        profile_url: &links.profile_url,
    }
    .render()
}

fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    if path == "/" || path.is_empty() {
        return format!("{base}/");
    }
    let path = path.trim_start_matches('/');
    format!("{base}/{path}")
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b'/' => out.push_str("%2F"),
            b':' => out.push_str("%3A"),
            b'?' => out.push_str("%3F"),
            b'#' => out.push_str("%23"),
            b'[' => out.push_str("%5B"),
            b']' => out.push_str("%5D"),
            b'@' => out.push_str("%40"),
            b'!' => out.push_str("%21"),
            b'$' => out.push_str("%24"),
            b'&' => out.push_str("%26"),
            b'\'' => out.push_str("%27"),
            b'(' => out.push_str("%28"),
            b')' => out.push_str("%29"),
            b'*' => out.push_str("%2A"),
            b'+' => out.push_str("%2B"),
            b',' => out.push_str("%2C"),
            b';' => out.push_str("%3B"),
            b'=' => out.push_str("%3D"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_login_and_profile_urls() {
        let links = auth_links(
            "http://identity.example/",
            "http://store.example/",
            "/products/SIGMA-RACER",
        );
        assert!(links.sign_in_url.contains("/auth/login?app_uri="));
        assert!(
            links
                .sign_in_url
                .contains("app_uri=http%3A%2F%2Fstore.example%2Fproducts%2FSIGMA-RACER")
        );
        assert!(
            links
                .sign_in_url
                .contains("redirect_uri=http%3A%2F%2Fidentity.example%2Fauth%2Fcallback")
        );
        assert!(links.profile_url.contains("/profile?return_url="));
        assert!(links.register_url.contains("/register?return_url="));
        assert_eq!(links.identity_base_url, "http://identity.example/");
    }

    #[test]
    fn root_return_path_keeps_trailing_slash() {
        let links = auth_links("http://identity.example", "http://store.example", "/");
        assert!(
            links
                .sign_in_url
                .contains("app_uri=http%3A%2F%2Fstore.example%2F")
        );
    }

    #[test]
    fn renders_widget_with_ids() {
        let links = auth_links("http://identity.example", "http://store.example", "/");
        let html = render_auth_nav(&links).expect("render");
        assert!(html.contains("id=\"store-nav-auth\""));
        assert!(html.contains("id=\"store-nav-signed-out\""));
        assert!(html.contains("id=\"store-nav-signed-in\""));
        assert!(html.contains("id=\"store-nav-welcome\""));
        assert!(html.contains("store-nav-auth.js"));
        // Sign-out button must not appear in the nav widget.
        assert!(!html.contains("Sign out"));
    }
}
