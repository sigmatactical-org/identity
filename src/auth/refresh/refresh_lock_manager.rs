//! [`RefreshLockManager`].

use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub(crate) struct RefreshLockManager {
    refreshing: Arc<Mutex<HashSet<String>>>,
    pub(crate) remaining_secs_threshold: u64,
}

impl RefreshLockManager {
    /// Refresh policy: renew when less than the threshold remains.
    pub(crate) fn new(remaining_secs_threshold: u64) -> Self {
        Self {
            refreshing: Arc::new(Mutex::new(HashSet::new())),
            remaining_secs_threshold,
        }
    }

    /// Claim the in-flight refresh slot for `userid`, or reject when one is pending.
    pub(crate) async fn try_acquire(&self, userid: &str) -> Result<(), Response> {
        let mut users = self.refreshing.lock().await;
        if !users.insert(userid.to_string()) {
            return Err((StatusCode::CONFLICT, "Refresh pending...").into_response());
        }
        Ok(())
    }

    pub(crate) async fn release(&self, userid: &str) {
        self.refreshing.lock().await.remove(userid);
    }
}
