//! [`RealmAccessClaim`].

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct RealmAccessClaim {
    #[serde(default)]
    pub(crate) roles: Vec<String>,
}
