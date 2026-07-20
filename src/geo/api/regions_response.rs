//! [`RegionsResponse`].

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RegionsResponse {
    pub prompt: String,
    pub regions: Vec<&'static str>,
}
