//! [`ProfileViewTemplate`].

use super::ProfileViewRow;

use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "profile.html")]
pub(crate) struct ProfileViewTemplate {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
    pub(crate) return_url: String,
    pub(crate) edit_url: String,
    pub(crate) logout_url: String,
    pub(crate) addresses_url: String,
    pub(crate) payments_url: String,
    pub(crate) orders_url: String,
    pub(crate) saved: bool,
    pub(crate) rows: Vec<ProfileViewRow>,
}
