//! Environment-variable helpers shared by the runner and the flow driver.

/// Parse an env var as `f64`, falling back to `default` when unset or invalid.
pub fn env_f64(name: &str, default: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Whether an env var is set to one of the falsy spellings (`0`, `false`, `FALSE`).
pub fn env_is_false(name: &str) -> bool {
    std::env::var(name)
        .map(|v| matches!(v.as_str(), "0" | "false" | "FALSE"))
        .unwrap_or(false)
}

/// Regexes for scraping an auto-submit HTML form (action + hidden fields).
pub mod form_regex {
    use std::sync::LazyLock;

    use regex::Regex;

    /// The `action="..."` attribute of a form.
    pub static ACTION: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r#"action="([^"]+)""#).expect("valid action regex"));

    /// A `name="..." value="..."` input pair.
    pub static FIELD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"name="([^"]+)"\s+value="([^"]+)""#).expect("valid field regex")
    });
}
