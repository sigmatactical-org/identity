//! [`AuthNavTemplate`].

use askama::Template;

#[derive(Template)]
#[template(path = "auth_nav.html")]
pub(crate) struct AuthNavTemplate<'a> {
    pub(crate) sign_in_url: &'a str,
    pub(crate) identity_base_url: &'a str,
    pub(crate) profile_url: &'a str,
}
