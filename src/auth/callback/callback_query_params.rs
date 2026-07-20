//! [`CallbackQueryParams`].

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct CallbackQueryParams {
    pub(crate) code: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) state: String,
}
