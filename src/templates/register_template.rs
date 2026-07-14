//! [`RegisterTemplate`].

#[allow(unused_imports)]
use super::*;
use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "register.html")]
pub(crate) struct RegisterTemplate {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
    pub(crate) return_url: String,
    pub(crate) email: String,
    pub(crate) username: String,
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    pub(crate) error: String,
    pub(crate) human_check_enabled: bool,
    pub(crate) human_check_challenge_url: String,
}
