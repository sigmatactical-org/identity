//! [`ProfileInput`].

/// All editable profile fields, as submitted by the profile edit form.
#[derive(Debug, Default, Clone)]
pub(crate) struct ProfileInput {
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub phone: String,
    pub street: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country: String,
    pub birthdate: String,
    pub company: String,
}
