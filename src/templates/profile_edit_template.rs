//! [`ProfileEditTemplate`].

use askama::Template;
use sigma_theme::nav::SiteHeader;

use super::CountryOption;

#[derive(Template)]
#[template(path = "profile_edit.html")]
pub(crate) struct ProfileEditTemplate<'a> {
    pub(crate) site_header: SiteHeader,
    pub(crate) site_nav: String,
    pub(crate) copyright_years: String,
    pub(crate) return_url: &'a str,
    pub(crate) cancel_url: &'a str,
    pub(crate) error: &'a str,
    pub(crate) username: &'a str,
    pub(crate) email: &'a str,
    pub(crate) first_name: &'a str,
    pub(crate) last_name: &'a str,
    pub(crate) phone: &'a str,
    pub(crate) street: &'a str,
    pub(crate) city: &'a str,
    pub(crate) postal_code: &'a str,
    pub(crate) birthdate: &'a str,
    pub(crate) company: &'a str,
    pub(crate) countries: Vec<CountryOption<'a>>,
    pub(crate) region: &'a str,
}
