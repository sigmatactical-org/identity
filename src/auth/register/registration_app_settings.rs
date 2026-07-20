//! [`RegistrationAppSettings`].

use super::super::allowlist::UriAllowlist;

#[derive(Clone, Debug)]
pub struct RegistrationAppSettings {
    pub(crate) return_uris: UriAllowlist,
}
impl RegistrationAppSettings {
    /// Registration settings from return-URL allowlist entries.
    pub fn new(return_uri_entries: Vec<String>) -> Self {
        Self {
            return_uris: UriAllowlist::new(return_uri_entries),
        }
    }

    /// Whether a post-registration return URL is allowed.
    pub(crate) fn is_return_url_allowed(&self, return_url: &str) -> bool {
        self.return_uris.is_allowed(return_url)
    }
}
