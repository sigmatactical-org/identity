//! [`AdminUsersTemplate`].

#[allow(unused_imports)]
use super::*;
use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "admin_users.html")]
pub(crate) struct AdminUsersTemplate {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
    pub(crate) search: String,
    pub(crate) has_prev: bool,
    pub(crate) has_next: bool,
    pub(crate) prev_page: u32,
    pub(crate) next_page: u32,
    pub(crate) users: Vec<AdminUserRow>,
    pub(crate) service_accounts: Vec<AdminServiceAccountRow>,
    pub(crate) notice: String,
}
