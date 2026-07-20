//! Reusable Sigma sign-in / "Welcome" navbar widget shared across Sigma web
//! services (store, cart, ...).
//!
//! [`auth_links`] builds identity URLs for a given app and return path;
//! [`render_auth_nav`] renders the client-side auth widget consumed by
//! `sigma-theme`'s `store-nav-auth.js` (it keeps the same element IDs, so the
//! shared JS toggles signed-in/out state without changes). The widget shows a
//! "Sign in" button when signed out and a "Welcome, {username}" link to the
//! profile page when signed in; sign-out lives on the profile page.

#![forbid(unsafe_code)]

mod auth_links;
mod auth_nav_template;
pub use auth_links::AuthLinks;
pub(crate) use auth_nav_template::AuthNavTemplate;

use askama::Template;
use urlencoding::encode as percent_encode;

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
