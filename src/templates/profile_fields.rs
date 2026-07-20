//! [`ProfileFields`].

/// All profile field values shared by the read-only view and the edit form.
/// Empty strings represent unfilled fields.
#[derive(Debug, Default, Clone)]
pub struct ProfileFields {
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
