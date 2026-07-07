//! Reusable Sigma sign-in / "Welcome" navbar widget shared across Sigma web
//! services (store, cart, ...).
//!
//! [`auth_links`] builds the identity/contact URLs for a given app and return
//! path; [`render_auth_nav`] renders the client-side auth widget consumed by
//! `sigma-theme`'s `store-nav-auth.js` (it keeps the same element IDs, so the
//! shared JS toggles signed-in/out state without changes). The widget shows a
//! "Sign in" button when signed out and a "Welcome, {username}" link to the
//! profile page when signed in; sign-out lives on the profile page.

use askama::Template;
use sigma_cart_nav::render_cart_nav;
use sigma_contact_nav::render_contact_nav;

/// Resolved identity / contact URLs for a service's navbar.
pub struct AuthLinks {
    /// Identity login URL that returns the user to the calling app.
    pub sign_in_url: String,
    /// Identity profile page URL for the signed-in user.
    pub profile_url: String,
    /// Identity service public base URL (trailing slash), used by the widget JS.
    pub identity_base_url: String,
    /// Contact service URL that returns the user to the calling app.
    pub contact_us_url: String,
}

/// Build the identity/contact URLs that return the user to `return_path` on the
/// calling app.
///
/// - `identity_base`: public base URL of the identity service.
/// - `app_base`: public base URL of the calling app (used to build `app_uri`).
/// - `contact_base`: public base URL of the contact service.
/// - `return_path`: path on the calling app to return to after sign-in.
#[must_use]
pub fn auth_links(
    identity_base: &str,
    app_base: &str,
    contact_base: &str,
    return_path: &str,
) -> AuthLinks {
    let app_uri = join_url(app_base, return_path);
    let identity_root = identity_base.trim_end_matches('/');
    let contact_root = contact_base.trim_end_matches('/');
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
    let contact_us_url = format!(
        "{contact_root}/contact?return_url={}",
        percent_encode(&app_uri)
    );

    AuthLinks {
        sign_in_url,
        profile_url,
        identity_base_url: format!("{identity_root}/"),
        contact_us_url,
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

#[derive(Template)]
#[template(path = "site_nav.html")]
struct SiteNavTemplate<'a> {
    leading_html: &'a str,
    auth_nav: &'a str,
    cart_nav: &'a str,
    contact_nav: &'a str,
}

/// Render the standard Sigma header actions: optional leading link, sign-in /
/// welcome widget, cart icon, and optionally a contact-us button.
///
/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_site_nav(
    links: &AuthLinks,
    cart_url: &str,
    cart_count: u32,
    show_contact_us: bool,
    leading_html: &str,
) -> Result<String, askama::Error> {
    let auth_nav = render_auth_nav(links)?;
    let cart_nav = render_cart_nav(cart_url, cart_count)?;
    let contact_nav = if show_contact_us {
        render_contact_nav(&links.contact_us_url)?
    } else {
        String::new()
    };
    SiteNavTemplate {
        leading_html,
        auth_nav: &auth_nav,
        cart_nav: &cart_nav,
        contact_nav: &contact_nav,
    }
    .render()
}

/// Inputs for [`render_app_site_nav`].
pub struct AppSiteNav<'a> {
    pub identity_base: &'a str,
    pub app_base: &'a str,
    pub contact_base: &'a str,
    pub cart_url: &'a str,
    pub cart_count: u32,
    pub return_path: &'a str,
    pub show_contact_us: bool,
    pub leading_html: &'a str,
}

/// Convenience wrapper around [`auth_links`] and [`render_site_nav`].
///
/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render_app_site_nav(input: &AppSiteNav<'_>) -> Result<String, askama::Error> {
    let links = auth_links(
        input.identity_base,
        input.app_base,
        input.contact_base,
        input.return_path,
    );
    render_site_nav(
        &links,
        input.cart_url,
        input.cart_count,
        input.show_contact_us,
        input.leading_html,
    )
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
    fn builds_login_profile_and_contact_urls() {
        let links = auth_links(
            "http://identity.example/",
            "http://store.example/",
            "http://contact.example/",
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
        assert!(links.contact_us_url.contains("/contact?return_url="));
        assert_eq!(links.identity_base_url, "http://identity.example/");
    }

    #[test]
    fn root_return_path_keeps_trailing_slash() {
        let links = auth_links(
            "http://identity.example",
            "http://store.example",
            "http://contact.example",
            "/",
        );
        assert!(
            links
                .sign_in_url
                .contains("app_uri=http%3A%2F%2Fstore.example%2F")
        );
    }

    #[test]
    fn renders_widget_with_ids() {
        let links = auth_links(
            "http://identity.example",
            "http://store.example",
            "http://contact.example",
            "/",
        );
        let html = render_auth_nav(&links).expect("render");
        assert!(html.contains("id=\"store-nav-auth\""));
        assert!(html.contains("id=\"store-nav-signed-out\""));
        assert!(html.contains("id=\"store-nav-signed-in\""));
        assert!(html.contains("id=\"store-nav-welcome\""));
        assert!(html.contains("store-nav-auth.js"));
        // Sign-out button must not appear in the nav widget.
        assert!(!html.contains("Sign out"));
    }

    #[test]
    fn site_nav_includes_auth_cart_and_contact() {
        let links = auth_links(
            "http://identity.example",
            "http://store.example",
            "http://contact.example",
            "/",
        );
        let html = render_site_nav(&links, "http://cart.example/", 2, true, "").expect("render");
        assert!(html.contains("store-nav-auth"));
        assert!(html.contains("href=\"http://cart.example/\""));
        assert!(html.contains("Contact us"));
        assert!(html.contains(">2</span>"));
    }

    #[test]
    fn site_nav_can_hide_contact_us() {
        let links = auth_links(
            "http://identity.example",
            "http://contact.example",
            "http://contact.example",
            "/contact",
        );
        let html = render_site_nav(&links, "http://cart.example/", 0, false, "").expect("render");
        assert!(html.contains("store-nav-auth"));
        assert!(!html.contains("Contact us"));
    }
}
