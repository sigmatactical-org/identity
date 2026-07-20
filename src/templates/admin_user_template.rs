//! [`AdminUserTemplate`].

use super::ProfileViewRow;

use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "admin_user.html")]
pub(crate) struct AdminUserTemplate {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
    pub(crate) user_id: String,
    pub(crate) username: String,
    pub(crate) enabled: bool,
    pub(crate) email_verified: bool,
    pub(crate) notice: String,
    pub(crate) rows: Vec<ProfileViewRow>,
}
