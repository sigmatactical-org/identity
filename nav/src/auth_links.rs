//! [`AuthLinks`].

/// Resolved identity URLs for a service's navbar.
pub struct AuthLinks {
    /// Identity login URL that returns the user to the calling app.
    pub sign_in_url: String,
    /// Identity profile page URL for the signed-in user.
    pub profile_url: String,
    /// Identity registration page URL that returns the user to the calling app.
    pub register_url: String,
    /// Identity service public base URL (trailing slash), used by the widget JS.
    pub identity_base_url: String,
}
