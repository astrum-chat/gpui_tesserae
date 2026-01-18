use gpui::{App, Window};

use crate::{primitives::input, theme::ThemeExt};

pub fn init(cx: &mut App) {
    input::init(cx);
}

pub fn init_for_window(window: &mut Window, cx: &mut App) {
    window.set_rem_size(cx.get_theme().layout.text.base_size);
}
