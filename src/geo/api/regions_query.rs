//! [`RegionsQuery`].

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RegionsQuery {
    pub country: String,
}
