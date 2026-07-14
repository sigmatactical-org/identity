//! [`ExpFieldInJWT`].

#[allow(unused_imports)]
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ExpFieldInJWT {
    pub(crate) exp: u64,
}
