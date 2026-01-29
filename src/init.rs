use gpui::{App, Window};

use crate::{
    components::select,
    primitives::{input, selectable_text},
    theme::ThemeExt,
};

/// Initializes global tesserae state. Call once at application startup.
pub fn init(cx: &mut App) {
    input::init(cx);
    selectable_text::init(cx);
    select::init(cx);
}

/// Initializes per-window tesserae state. Call for each new window.
pub fn init_for_window(window: &mut Window, cx: &mut App) {
    window.set_rem_size(cx.get_theme().layout.text.base_size);
}
