//! [`ServiceAccountRow`].

#[allow(unused_imports)]
use super::*;

/// A client's service-account user, labeled with the owning client id.
pub(crate) struct ServiceAccountRow {
    pub client_id: String,
    pub user: UserSummary,
}
