use gpui::{ElementId, SharedString};

pub trait ElementIdExt {
    fn with_suffix(&self, suffix: impl Into<SharedString>) -> ElementId;
}

impl ElementIdExt for ElementId {
    fn with_suffix(&self, suffix: impl Into<SharedString>) -> ElementId {
        ElementId::NamedChild(Box::new(self.clone()), suffix.into())
    }
}
