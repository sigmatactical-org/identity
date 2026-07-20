//! [`AppConfigurationState`].

use axum::extract::FromRef;

use super::{LoginAppSettings, LogoutAppSettings};

#[derive(Clone)]
pub(crate) struct AppConfigurationState {
    pub(crate) login_app_settings: LoginAppSettings,
    pub(crate) logout_app_settings: LogoutAppSettings,
}

impl FromRef<AppConfigurationState> for LoginAppSettings {
    fn from_ref(app_state: &AppConfigurationState) -> Self {
        app_state.login_app_settings.clone()
    }
}

impl FromRef<AppConfigurationState> for LogoutAppSettings {
    fn from_ref(app_state: &AppConfigurationState) -> LogoutAppSettings {
        app_state.logout_app_settings.clone()
    }
}
