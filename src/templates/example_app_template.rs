//! [`ExampleAppTemplate`].

use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "exampleapp.html")]
pub(crate) struct ExampleAppTemplate {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
}
