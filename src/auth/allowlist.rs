use tracing::debug;

/// Exact-match and trailing-wildcard (`*`) URI allowlist.
#[derive(Clone, Debug)]
pub(crate) struct UriAllowlist {
    exact: Vec<String>,
    prefix: Vec<String>,
}

impl UriAllowlist {
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

    pub(crate) fn is_allowed(&self, uri: &str) -> bool {
        if self.exact.iter().any(|e| e == uri) {
            return true;
        }
        self.prefix.iter().any(|p| uri.starts_with(p))
    }
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
}
