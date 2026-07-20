//! [`ExtraProxyRoute`].

/// A path prefix rewritten to a different proxy target.
#[derive(Debug, Clone)]
pub(crate) struct ExtraProxyRoute {
    pub(crate) path: String,
    pub(crate) target: String,
}
