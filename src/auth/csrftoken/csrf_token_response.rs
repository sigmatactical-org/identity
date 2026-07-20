//! [`CsrfTokenResponse`].

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CsrfTokenResponse {
    pub(crate) token: String,
}
