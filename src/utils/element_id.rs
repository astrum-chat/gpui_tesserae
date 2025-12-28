use std::sync::Arc;

use gpui::{ElementId, SharedString};

pub trait ElementIdExt {
    fn with_suffix(&self, suffix: impl Into<SharedString>) -> ElementId;
}

impl ElementIdExt for ElementId {
    fn with_suffix(&self, suffix: impl Into<SharedString>) -> ElementId {
        ElementId::NamedChild(Arc::new(self.clone()), suffix.into())
    }
}
