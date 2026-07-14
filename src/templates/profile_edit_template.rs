//! [`ProfileEditTemplate`].

#[allow(unused_imports)]
use super::*;
use askama::Template;
use sigma_theme::nav::SiteHeader;

#[derive(Template)]
#[template(path = "profile_edit.html")]
pub(crate) struct ProfileEditTemplate {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
    pub(crate) return_url: String,
    pub(crate) cancel_url: String,
    pub(crate) error: String,
    pub(crate) username: String,
    pub(crate) email: String,
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    pub(crate) phone: String,
    pub(crate) street: String,
    pub(crate) city: String,
    pub(crate) postal_code: String,
    pub(crate) birthdate: String,
    pub(crate) company: String,
    pub(crate) countries: Vec<CountryOption>,
    pub(crate) region: String,
}
