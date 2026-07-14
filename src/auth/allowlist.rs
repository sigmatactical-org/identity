use axum::http::HeaderValue;
use tracing::debug;

/// Exact-match and trailing-wildcard (`*`) URI allowlist.
#[derive(Clone, Debug)]
pub(crate) struct UriAllowlist {
    exact: Vec<String>,
    prefix: Vec<String>,
}

impl UriAllowlist {
    /// Allowlist from exact-match URI entries.
    pub(crate) fn new(entries: Vec<String>) -> Self {
        let mut exact = Vec::new();
        let mut prefix = Vec::new();
        for entry in entries {
            if entry.ends_with('*') {
                let prefix_entry = entry[..entry.len() - 1].to_string();
                debug!("Allowed URI prefix: {prefix_entry}");
                prefix.push(prefix_entry);
            } else {
                debug!("Allowed URI: {entry}");
                exact.push(entry);
            }
        }
        Self { exact, prefix }
    }

    /// Whether `uri` is on the allowlist.
    pub(crate) fn is_allowed(&self, uri: &str) -> bool {
        if self.exact.iter().any(|e| e == uri) {
            return true;
        }
        self.prefix.iter().any(|p| uri.starts_with(p))
    }
}

/// Derive browser origins (`scheme://host:port`) from allowlist entries for CORS.
pub(crate) fn browser_origins_from_entries(entries: &[String]) -> Vec<HeaderValue> {
    let mut origins = Vec::new();
    for entry in entries {
        let trimmed = entry.trim();
        let base = trimmed
            .strip_suffix('*')
            .unwrap_or(trimmed)
            .trim_end_matches('/');
        let Ok(url) = url::Url::parse(base) else {
            continue;
        };
        let origin = url.origin().ascii_serialization();
        if let Ok(value) = HeaderValue::from_str(&origin) {
            origins.push(value);
        }
    }
    origins.sort();
    origins.dedup();
    origins
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_wildcard() {
        let list = UriAllowlist::new(vec![
            "http://example.com".to_string(),
            "http://example.org/app/*".to_string(),
        ]);
        assert!(list.is_allowed("http://example.com"));
        assert!(list.is_allowed("http://example.org/app/"));
        assert!(!list.is_allowed("http://example.com/"));
    }

    #[test]
    fn browser_origins_strip_wildcards() {
        let origins = browser_origins_from_entries(&[
            "http://store.example:30080/*".to_string(),
            "http://identity.example/".to_string(),
        ]);
        assert_eq!(origins.len(), 2);
        assert_eq!(origins[0].to_str().unwrap(), "http://identity.example");
        assert_eq!(origins[1].to_str().unwrap(), "http://store.example:30080");
    }
}
