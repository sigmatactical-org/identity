//! [`AdminActionQuery`].

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct AdminActionQuery {
    #[serde(default)]
    pub(crate) notice: String,
}
