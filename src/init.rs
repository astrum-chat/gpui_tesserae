use gpui::{App, Window};

use crate::{
    components::select,
    primitives::{input, selectable_text},
    theme::ThemeExt,
};

/// Initializes global tesserae state. Call once at application startup.
pub fn init(cx: &mut App) {
    init_fonts(cx).expect("Could not initialize fonts!");

    input::init(cx);
    selectable_text::init(cx);
    select::init(cx);
}

/// Initializes per-window tesserae state. Call for each new window.
pub fn init_for_window(window: &mut Window, cx: &mut App) {
    window.set_rem_size(cx.get_theme().layout.text.base_size);
}

/// Initializes font assets.
pub fn init_fonts(cx: &mut App) -> gpui::Result<()> {
    let font_paths = cx.asset_source().list("fonts")?;
    let mut embedded_fonts = Vec::new();

    for font_path in font_paths {
        if !font_path.ends_with(".ttf") {
            continue;
        }

        let Some(font_bytes) = cx.asset_source().load(&font_path)? else {
            continue;
        };

        embedded_fonts.push(font_bytes);
    }

    cx.text_system().add_fonts(embedded_fonts)
}
