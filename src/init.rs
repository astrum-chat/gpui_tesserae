use gpui::{App, Window};

use crate::{
    primitives::input,
    theme::{ActiveVariantId, ThemeExt},
};

pub fn init(cx: &mut App) {
    cx.set_global::<ActiveVariantId>(ActiveVariantId(0));
    input::init(cx);
}

pub fn init_for_window(window: &mut Window, cx: &mut App) {
    window.set_rem_size(cx.get_theme().layout.text.base_size);
}
