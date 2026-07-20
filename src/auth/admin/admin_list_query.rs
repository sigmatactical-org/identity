//! [`AdminListQuery`].

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct AdminListQuery {
    #[serde(default)]
    pub(crate) q: String,
    #[serde(default)]
    pub(crate) page: u32,
    #[serde(default)]
    pub(crate) notice: String,
}
