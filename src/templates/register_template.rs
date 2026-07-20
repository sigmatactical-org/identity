//! [`RegisterTemplate`].

use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "register.html")]
pub(crate) struct RegisterTemplate<'a> {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
    pub(crate) return_url: &'a str,
    pub(crate) email: &'a str,
    pub(crate) username: &'a str,
    pub(crate) first_name: &'a str,
    pub(crate) last_name: &'a str,
    pub(crate) error: &'a str,
    pub(crate) human_check_enabled: bool,
    pub(crate) human_check_challenge_url: &'a str,
}
