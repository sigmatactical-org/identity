//! [`CountryOption`].

/// A country `<option>` for the edit form's country dropdown.
pub(crate) struct CountryOption<'a> {
    pub(crate) name: &'a str,
    pub(crate) selected: bool,
}
