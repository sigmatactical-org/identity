//! [`RegistrationMode`].

/// Registration finalization policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistrationMode {
    /// Enable the account immediately after create (dev convenience only).
    AutoApprove,
    /// Send Keycloak VERIFY_EMAIL and wait for `/register/verified`.
    VerifyEmail,
}
