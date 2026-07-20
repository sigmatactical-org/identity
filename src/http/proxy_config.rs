//! [`ProxyConfig`].

use std::str::FromStr;

use anyhow::{Result, anyhow};
use axum::http::Uri;

use super::ExtraProxyRoute;

#[derive(Debug, Clone)]
pub(crate) struct ProxyConfig {
    base_url: String,
    pub(crate) cookie_name: String,
    pub(crate) extra_routes: Vec<ExtraProxyRoute>,
}

impl ProxyConfig {
    /// Map an incoming path onto the proxy target, honoring extra routes.
    pub(crate) fn rewrite_uri(&self, uri: &str) -> String {
        self.extra_routes
            .iter()
            .find_map(|route| {
                uri.strip_prefix(route.path.as_str())
                    .map(|path| format!("{}{path}", route.target))
            })
            .unwrap_or_else(|| format!("{}{uri}", self.base_url))
    }

    /// Validate the proxy target and parse `path=>target` rewrite rules,
    /// longest path first so the most specific rule wins.
    pub(crate) fn try_init(
        base_url: String,
        cookie_name: &str,
        extra_routes: Vec<String>,
    ) -> Result<Self> {
        let uri = Uri::from_str(base_url.as_str())?;
        if uri.host().is_none() {
            return Err(anyhow!("Missing host"));
        }
        let mut extra_routes: Vec<ExtraProxyRoute> = extra_routes
            .into_iter()
            .filter_map(|s| {
                s.split_once("=>").map(|(path, target)| ExtraProxyRoute {
                    path: path.to_string(),
                    target: target.to_string(),
                })
            })
            .collect();
        extra_routes.sort_by_key(|b| std::cmp::Reverse(b.path.len()));
        Ok(Self {
            base_url,
            cookie_name: cookie_name.to_string(),
            extra_routes,
        })
    }
}
