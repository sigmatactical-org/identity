//! [`ProfileViewRow`].

#[allow(unused_imports)]
use super::*;

/// A single label/value pair for read-only profile and admin detail views.
pub(crate) struct ProfileViewRow {
    pub label: String,
    pub value: String,
}
