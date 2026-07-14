//! [`CountryOption`].

#[allow(unused_imports)]
use super::*;

/// A country `<option>` for the edit form's country dropdown.
pub(crate) struct CountryOption {
    pub(crate) name: String,
    pub(crate) selected: bool,
}
