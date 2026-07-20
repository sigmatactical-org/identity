//! [`AppBuildOptions`].

use crate::auth::{AdminDeps, AppConfigurationState, ProfileDeps, RegistrationDeps};

/// Toggles for optional route groups when building the app.
pub(crate) struct AppBuildOptions {
    pub remaining_secs_threshold: u64,
    pub app_config: AppConfigurationState,
    pub registration: Option<RegistrationDeps>,
    pub profile: Option<ProfileDeps>,
    pub admin: Option<AdminDeps>,
}
