#![warn(missing_docs)]

//! Primitive UI components built on GPUI.

/// Text input components with single-line and multiline support.
pub mod input;

/// Selectable text component for read-only text with selection support.
pub mod selectable_text;

/// Inline text flow container with character-level selection support.
pub mod selectable_layout;

mod utils;

mod extensions;

/// Initialize all gpui_primitives components (key bindings, etc.).
/// Calls `input::init`, `selectable_text::init`, and `selectable_layout::init`.
pub fn init(cx: &mut gpui::App) {
    input::init(cx);
    selectable_text::init(cx);
    selectable_layout::init(cx);
}
