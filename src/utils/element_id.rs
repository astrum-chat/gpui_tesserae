use std::sync::Arc;

use gpui::{ElementId, SharedString};

/// Extension trait for creating derived element IDs.
pub trait ElementIdExt {
    /// Creates a new element ID by appending a suffix to this ID.
    fn with_suffix(&self, suffix: impl Into<SharedString>) -> ElementId;
}

impl ElementIdExt for ElementId {
    fn with_suffix(&self, suffix: impl Into<SharedString>) -> ElementId {
        ElementId::NamedChild(Arc::new(self.clone()), suffix.into())
    }
}
