//! [`ProfileForm`].

use serde::Deserialize;

use crate::auth::ProfileInput;
use crate::templates;

#[derive(Debug, Deserialize)]
pub(crate) struct ProfileForm {
    pub(crate) return_url: String,
    pub(crate) username: String,
    pub(crate) email: String,
    pub(crate) first_name: String,
    pub(crate) last_name: String,
    #[serde(default)]
    pub(crate) phone: String,
    #[serde(default)]
    pub(crate) street: String,
    #[serde(default)]
    pub(crate) city: String,
    #[serde(default)]
    pub(crate) region: String,
    #[serde(default)]
    pub(crate) postal_code: String,
    #[serde(default)]
    pub(crate) country: String,
    #[serde(default)]
    pub(crate) birthdate: String,
    #[serde(default)]
    pub(crate) company: String,
}

impl ProfileForm {
    /// The submitted values as template fields, for re-rendering the form.
    pub(crate) fn fields(&self) -> templates::ProfileFields {
        templates::ProfileFields {
            username: self.username.clone(),
            email: self.email.clone(),
            first_name: self.first_name.clone(),
            last_name: self.last_name.clone(),
            phone: self.phone.clone(),
            street: self.street.clone(),
            city: self.city.clone(),
            region: self.region.clone(),
            postal_code: self.postal_code.clone(),
            country: self.country.clone(),
            birthdate: self.birthdate.clone(),
            company: self.company.clone(),
        }
    }

    /// The submitted values, trimmed, as the Keycloak update payload.
    pub(crate) fn input(&self) -> ProfileInput {
        ProfileInput {
            username: self.username.trim().to_string(),
            email: self.email.trim().to_string(),
            first_name: self.first_name.trim().to_string(),
            last_name: self.last_name.trim().to_string(),
            phone: self.phone.trim().to_string(),
            street: self.street.trim().to_string(),
            city: self.city.trim().to_string(),
            region: self.region.trim().to_string(),
            postal_code: self.postal_code.trim().to_string(),
            country: self.country.trim().to_string(),
            birthdate: self.birthdate.trim().to_string(),
            company: self.company.trim().to_string(),
        }
    }

    /// First validation failure, if any.
    pub(crate) fn validate(&self) -> Option<String> {
        if self.username.trim().is_empty() {
            return Some("Username is required.".into());
        }
        if self.email.trim().is_empty() || !self.email.contains('@') {
            return Some("A valid email address is required.".into());
        }
        None
    }
}
