use gpui::{App, Window};

use crate::{Assets, theme::ThemeExt};

/// Initializes global tesserae state. Call once at application startup.
pub fn init(cx: &mut App) {
    Assets::init_fonts(cx).expect("Could not initialize fonts!");

    gpui_primitives::init(cx);
}

/// Initializes per-window tesserae state. Call for each new window.
pub fn init_for_window(window: &mut Window, cx: &mut App) {
    window.set_rem_size(cx.get_theme().layout.text.base_size);
}
