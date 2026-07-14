//! [`RegisterSuccessTemplate`].

#[allow(unused_imports)]
use super::*;
use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "register_success.html")]
pub(crate) struct RegisterSuccessTemplate {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
    pub(crate) return_url: String,
    pub(crate) auto_approved: bool,
}
