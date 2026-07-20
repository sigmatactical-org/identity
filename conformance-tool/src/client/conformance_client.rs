//! [`ConformanceClient`].

use reqwest::Client;

/// HTTP client for the OpenID Foundation conformance suite API.
pub struct ConformanceClient {
    pub(super) client: Client,
    pub(super) server: String,
    pub(super) host_header: Option<String>,
}
