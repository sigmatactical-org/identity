//! [`IndexTemplate`].

use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "index.html")]
pub(crate) struct IndexTemplate {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
    pub(crate) signed_in: bool,
    pub(crate) is_admin: bool,
    pub(crate) profile_url: String,
    pub(crate) register_url: String,
    pub(crate) sign_in_url: String,
    pub(crate) addresses_url: String,
    pub(crate) payments_url: String,
    pub(crate) orders_url: String,
}
