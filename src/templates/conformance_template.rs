//! [`ConformanceTemplate`].

#[allow(unused_imports)]
use super::*;
use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "conformance.html")]
pub(crate) struct ConformanceTemplate {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
}
