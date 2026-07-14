//! [`RegionsQuery`].

#[allow(unused_imports)]
use super::*;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RegionsQuery {
    pub country: String,
}
