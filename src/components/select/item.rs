use gpui::{App, IntoElement, SharedString, Window};

pub trait SelectItem {
    type Value;

    fn name(&self) -> SharedString;

    fn value(&self) -> &Self::Value;

    #[allow(unused)]
    fn display(&self, window: &mut Window, cx: &App) -> impl IntoElement {
        self.name().into_any_element()
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
