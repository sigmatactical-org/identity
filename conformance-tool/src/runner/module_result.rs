//! [`ModuleResult`].

/// Outcome of one conformance test module.
pub struct ModuleResult {
    pub name: String,
    pub result: String,
    pub module_id: String,
}
