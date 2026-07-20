//! [`SessionSetup`].

use anyhow::Result;
use cookie::Key;
use time::Duration;
use tower_sessions::{Expiry, SessionManagerLayer, SessionStore, service::PrivateCookie};
use tracing::debug;

use super::SameSiteSetting;

#[derive(Debug, Clone)]
pub(crate) struct SessionSetup {
    pub(crate) secret: String,
    pub(crate) cookie_name: String,
    pub(crate) cookie_path: String,
    pub(crate) cookie_domain: Option<String>,
    pub(crate) ttl: Option<Duration>,
    pub(crate) secure_cookie: bool,
    pub(crate) same_site: SameSiteSetting,
}

impl SessionSetup {
    /// Session middleware over the given store.
    pub(crate) fn get_session_layer<Store: SessionStore>(
        &self,
        store: Store,
    ) -> Result<SessionManagerLayer<Store, PrivateCookie>> {
        debug!("Preparing session");
        let mut session_layer = SessionManagerLayer::new(store)
            .with_private(Key::from(self.secret.as_bytes()))
            .with_name(self.cookie_name.clone())
            .with_secure(self.secure_cookie)
            .with_path(self.cookie_path.clone())
            .with_same_site(self.same_site.to_tower_sessions_same_site())
            .with_expiry(Expiry::OnInactivity(
                self.ttl.unwrap_or_else(|| Duration::hours(1)),
            ));
        if let Some(domain) = self.cookie_domain.clone() {
            session_layer = session_layer.with_domain(domain);
        }

        Ok(session_layer)
    }
}
