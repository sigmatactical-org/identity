//! [`SameSiteSetting`].

#[derive(Debug, Clone)]
pub(crate) enum SameSiteSetting {
    None,
    Lax,
    Strict,
}

impl SameSiteSetting {
    /// Cookie SameSite mode from its env string.
    pub(crate) fn from_env_string(value: Option<String>) -> Self {
        match value.as_deref().map(str::to_lowercase).as_deref() {
            Some("none") => Self::None,
            Some("strict") => Self::Strict,
            _ => Self::Lax,
        }
    }

    /// Convert to the tower-sessions SameSite type.
    pub(crate) fn to_tower_sessions_same_site(&self) -> tower_sessions::cookie::SameSite {
        match self {
            Self::None => tower_sessions::cookie::SameSite::None,
            Self::Lax => tower_sessions::cookie::SameSite::Lax,
            Self::Strict => tower_sessions::cookie::SameSite::Strict,
        }
    }
}
