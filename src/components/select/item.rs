use gpui::{App, FocusHandle, IntoElement, SharedString, Window};

/// Defines how an item in a select menu is identified, accessed, and displayed.
pub trait SelectItem {
    /// The type of value this item represents.
    type Value;

    /// Returns a unique name used to identify this item.
    fn name(&self) -> SharedString;

    /// Returns a reference to the underlying value.
    fn value(&self) -> &Self::Value;

    /// Renders the item for display in the select menu.
    #[allow(unused)]
    fn display(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        self.name().into_any_element()
    }
}

/// Wrapper that holds a SelectItem along with its focus handle.
/// This is created internally when items are added to SelectState.
pub struct SelectItemEntry<I: SelectItem> {
    /// The wrapped item.
    pub item: I,
    /// Focus handle for keyboard navigation within the menu.
    pub focus_handle: FocusHandle,
}

impl<I: SelectItem> SelectItemEntry<I> {
    /// Creates a new entry wrapping the given item with a fresh focus handle.
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
