//! [`RegistrationOutcome`].

/// Result of adapter-specific post-registration handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RegistrationOutcome {
    /// Account is enabled and ready to sign in (dev auto-approve).
    Approved,
    /// Account awaits email verification.
    PendingEmailVerification,
}
