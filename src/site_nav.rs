//! Shared header navigation for identity HTML pages.

use sigma_identity_nav::render_app_site_nav;

use crate::config;

/// # Errors
///
/// Returns [`askama::Error`] when template rendering fails.
pub fn render(
    return_path: &str,
    show_contact_us: bool,
    leading_html: &str,
    cart_count: u32,
) -> Result<String, askama::Error> {
    render_app_site_nav(
        &config::public_base_url(),
        &config::public_base_url(),
        &config::contact_public_base_url(),
        &config::cart_public_base_url(),
        cart_count,
        return_path,
        show_contact_us,
        leading_html,
    )
}
