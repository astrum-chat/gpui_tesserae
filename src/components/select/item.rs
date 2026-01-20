use gpui::{App, FocusHandle, IntoElement, SharedString, Window};

pub trait SelectItem {
    type Value;

    fn name(&self) -> SharedString;

    fn value(&self) -> &Self::Value;

    #[allow(unused)]
    fn display(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        self.name().into_any_element()
    }
}

/// Wrapper that holds a SelectItem along with its focus handle.
/// This is created internally when items are added to SelectState.
pub struct SelectItemEntry<I: SelectItem> {
    pub item: I,
    pub focus_handle: FocusHandle,
}

impl<I: SelectItem> SelectItemEntry<I> {
    pub fn new(item: I, cx: &mut App) -> Self {
        Self {
            item,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl SelectItem for &'static str {
    type Value = &'static str;

    fn name(&self) -> SharedString {
        SharedString::new(self.to_string())
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

impl SelectItem for String {
    type Value = String;

    fn name(&self) -> SharedString {
        SharedString::new(self.to_string())
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

impl SelectItem for SharedString {
    type Value = SharedString;

    fn name(&self) -> SharedString {
        self.clone()
    }

    fn value(&self) -> &Self::Value {
        self
    }
}
