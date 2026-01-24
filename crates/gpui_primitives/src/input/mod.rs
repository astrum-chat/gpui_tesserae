use std::sync::Arc;

use gpui::{
    AbsoluteLength, App, CursorStyle, ElementId, Entity, FocusHandle, Focusable, Font, Hsla,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, Overflow, ParentElement, Refineable,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div, hsla, prelude::FluentBuilder,
    px, rgb, uniform_list,
};

mod cursor_blink;
mod elements;
mod selection;
mod state;
mod text_navigation;
/// Text transformation functions for input display (e.g., password masking).
pub mod text_transforms;

pub use cursor_blink::CursorBlink;
pub use state::{
    Backspace, Copy, Cut, Delete, DeleteToBeginningOfLine, DeleteToEndOfLine, DeleteToNextWordEnd,
    DeleteToPreviousWordStart, Down, End, Home, InputState, InsertNewline, InsertNewlineShift,
    Left, MapTextFn, MoveToEnd, MoveToEndOfLine, MoveToNextWord, MoveToPreviousWord, MoveToStart,
    MoveToStartOfLine, Paste, Quit, Redo, Right, SelectAll, SelectDown, SelectLeft, SelectRight,
    SelectToEnd, SelectToEndOfLine, SelectToNextWordEnd, SelectToPreviousWordStart, SelectToStart,
    SelectToStartOfLine, SelectUp, ShowCharacterPalette, Undo, Up, VisibleLineInfo, VisualLineInfo,
};

use crate::utils::{multiline_height, pixel_perfect_round, rgb_a};
use elements::{LineElement, TextElement, UniformListInputElement, WrappedLineElement};
use text_navigation::TextNavigation;

pub(crate) type TransformTextFn = Arc<dyn Fn(char) -> char + Send + Sync>;

// Re-export from utils for internal use by elements.rs
pub(crate) use crate::utils::WRAP_WIDTH_EPSILON;
pub(crate) use crate::utils::should_show_trailing_whitespace;

/// A text input element supporting single-line and multi-line editing with selection, clipboard, and undo/redo.
#[derive(IntoElement)]
pub struct Input {
    id: ElementId,
    state: Entity<InputState>,
    disabled: bool,
    line_clamp: usize,
    word_wrap: bool,
    newline_on_shift_enter: bool,
    placeholder: SharedString,
    placeholder_text_color: Option<Hsla>,
    selection_color: Option<Hsla>,
    transform_text: Option<TransformTextFn>,
    map_text: Option<MapTextFn>,
    style: StyleRefinement,
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Input {
    /// Creates a new input element with the given ID and state entity.
    pub fn new(id: impl Into<ElementId>, state: Entity<InputState>) -> Self {
        Self {
            id: id.into(),
            state,
            disabled: false,
            line_clamp: 1,
            word_wrap: false,
            newline_on_shift_enter: false,
            placeholder: "Type here...".into(),
            placeholder_text_color: None,
            selection_color: None,
            transform_text: None,
            map_text: None,
            style: StyleRefinement::default(),
        }
    }

    /// Sets the maximum number of visible lines before scrolling. Use `multiline()` for unlimited.
    pub fn line_clamp(mut self, line_clamp: usize) -> Self {
        self.line_clamp = line_clamp.max(1);
        self
    }

    /// Enables unlimited multiline input with vertical scrolling.
    pub fn multiline(mut self) -> Self {
        self.line_clamp = usize::MAX;
        self
    }

    /// Enables or disables word wrapping. Sets `line_clamp` to 1 if currently 0.
    pub fn word_wrap(mut self, enabled: bool) -> Self {
        if enabled && self.line_clamp == 0 {
            self.line_clamp = 1;
        }

        self.word_wrap = enabled;
        self
    }

    /// When enabled, Enter does nothing and Shift+Enter inserts a newline. Useful for chat inputs.
    pub fn newline_on_shift_enter(mut self, enabled: bool) -> Self {
        self.newline_on_shift_enter = enabled;
        self
    }

    /// Transforms each character for display without modifying the stored value. Useful for password fields.
    pub fn transform_text(
        mut self,
        transform: impl Fn(char) -> char + Send + Sync + 'static,
    ) -> Self {
        self.transform_text = Some(Arc::new(transform));
        self
    }

    /// Transform the text value whenever it changes.
    /// Unlike `transform_text`, this actually modifies the stored value.
    /// - `text`: The full text after the raw change was applied
    /// - `inserted_ranges`: Character ranges where new text was inserted, or None for deletion-only
    pub fn map_text(
        mut self,
        f: impl Fn(SharedString) -> SharedString + Send + Sync + 'static,
    ) -> Self {
        self.map_text = Some(Arc::new(f));
        self
    }

    /// Disables the input, preventing focus and interaction.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the maximum number of undo/redo history entries to keep.
    /// Defaults to 200.
    pub fn max_history(self, cx: &mut App, max: usize) -> Self {
        self.state.update(cx, |state, _| {
            state.set_max_history(max);
        });
        self
    }

    /// Sets the color for placeholder text.
    pub fn placeholder_text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.placeholder_text_color = Some(color.into());
        self
    }

    /// Sets the background color for selected text.
    pub fn selection_color(mut self, color: impl Into<Hsla>) -> Self {
        self.selection_color = Some(color.into());
        self
    }

    /// Sets the placeholder text shown when input is empty.
    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Returns the current placeholder text.
    pub fn get_placeholder(&self) -> &SharedString {
        &self.placeholder
    }

    /// Returns the current text value from state.
    pub fn read_text(&self, cx: &mut App) -> SharedString {
        self.state.read(cx).value()
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_multiline = self.line_clamp > 1;

        let text_style = &self.style.text;
        let font_size = match text_style.font_size.expect("font_size must be set") {
            AbsoluteLength::Pixels(px) => px,
            AbsoluteLength::Rems(rems) => rems.to_pixels(window.rem_size()),
        };
        let line_height = text_style
            .line_height
            .expect("line_height must be set")
            .to_pixels(font_size.into(), window.rem_size());
        let scale_factor = window.scale_factor();
        let line_height = pixel_perfect_round(line_height, scale_factor);

        let font = Font {
            family: text_style
                .font_family
                .clone()
                .expect("font_family must be set"),
            features: text_style.font_features.clone().unwrap_or_default(),
            fallbacks: text_style.font_fallbacks.clone(),
            weight: text_style.font_weight.unwrap_or_default(),
            style: text_style.font_style.unwrap_or_default(),
        };

        let map_text = self.map_text.clone();
        self.state.update(cx, |state, cx| {
            state.update_focus_state(window, cx);
            state.set_multiline_params(is_multiline, line_height, self.line_clamp);
            state.map_text = map_text;
        });

        let state = self.state.read(cx);

        let text_color = self
            .style
            .text
            .color
            .unwrap_or_else(|| rgb(0xE8E4FF).into());

        let placeholder_text_color = self
            .placeholder_text_color
            .unwrap_or_else(|| hsla(0., 0., 0., 0.2));
        let highlight_text_color = self
            .selection_color
            .unwrap_or_else(|| rgb_a(0x488BFF, 0.3).into());
        let cursor_visible = state.cursor_visible(cx);

        div()
            .id(self.id.clone())
            .map(|mut this| {
                this.style().refine(&self.style);
                this
            })
            .tab_index(0)
            .key_context("TextInput")
            .when(!self.disabled, |this| this.track_focus(&state.focus_handle))
            .cursor(if self.disabled {
                CursorStyle::OperationNotAllowed
            } else {
                CursorStyle::IBeam
            })
            // Basic navigation (universal)
            .on_action(window.listener_for(&self.state, InputState::backspace))
            .on_action(window.listener_for(&self.state, InputState::delete))
            .on_action(window.listener_for(&self.state, InputState::left))
            .on_action(window.listener_for(&self.state, InputState::right))
            .on_action(window.listener_for(&self.state, InputState::home))
            .on_action(window.listener_for(&self.state, InputState::end))
            // Basic selection (universal)
            .on_action(window.listener_for(&self.state, InputState::select_left))
            .on_action(window.listener_for(&self.state, InputState::select_right))
            .on_action(window.listener_for(&self.state, InputState::select_to_start_of_line))
            .on_action(window.listener_for(&self.state, InputState::select_to_end_of_line))
            // Clipboard & Undo
            .on_action(window.listener_for(&self.state, InputState::select_all))
            .on_action(window.listener_for(&self.state, InputState::copy))
            .on_action(window.listener_for(&self.state, InputState::cut))
            .on_action(window.listener_for(&self.state, InputState::paste))
            .on_action(window.listener_for(&self.state, InputState::undo))
            .on_action(window.listener_for(&self.state, InputState::redo))
            // Word navigation
            .on_action(window.listener_for(&self.state, InputState::move_to_previous_word))
            .on_action(window.listener_for(&self.state, InputState::move_to_next_word))
            .on_action(window.listener_for(&self.state, InputState::select_to_previous_word_start))
            .on_action(window.listener_for(&self.state, InputState::select_to_next_word_end))
            // Word deletion
            .on_action(window.listener_for(&self.state, InputState::delete_to_previous_word_start))
            .on_action(window.listener_for(&self.state, InputState::delete_to_next_word_end))
            // Line navigation
            .on_action(window.listener_for(&self.state, InputState::move_to_start_of_line))
            .on_action(window.listener_for(&self.state, InputState::move_to_end_of_line))
            // Document navigation
            .on_action(window.listener_for(&self.state, InputState::move_to_start))
            .on_action(window.listener_for(&self.state, InputState::move_to_end))
            .on_action(window.listener_for(&self.state, InputState::select_to_start))
            .on_action(window.listener_for(&self.state, InputState::select_to_end))
            // Line deletion
            .on_action(window.listener_for(&self.state, InputState::delete_to_beginning_of_line))
            .on_action(window.listener_for(&self.state, InputState::delete_to_end_of_line))
            // Character palette (macOS only)
            .on_action(window.listener_for(&self.state, InputState::show_character_palette))
            // Multiline-only actions
            .when(is_multiline, |this| {
                this.on_action(window.listener_for(&self.state, InputState::up))
                    .on_action(window.listener_for(&self.state, InputState::down))
                    .on_action(window.listener_for(&self.state, InputState::select_up))
                    .on_action(window.listener_for(&self.state, InputState::select_down))
                    .when(!self.newline_on_shift_enter, |this| {
                        this.on_action(window.listener_for(&self.state, InputState::insert_newline))
                    })
                    .when(self.newline_on_shift_enter, |this| {
                        this.on_action(
                            window.listener_for(&self.state, InputState::insert_newline_shift),
                        )
                    })
            })
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_down),
            )
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_up),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_up),
            )
            .on_mouse_move(window.listener_for(&self.state, InputState::on_mouse_move))
            .on_scroll_wheel(window.listener_for(&self.state, InputState::on_scroll_wheel))
            .when(is_multiline && !self.word_wrap, |this| {
                let font = font.clone();
                let line_count = state.line_count().max(1);
                let scroll_handle = state.scroll_handle.clone();
                let input_state = self.state.clone();
                let transform_text = self.transform_text.clone();
                let placeholder = self.placeholder.clone();
                let line_clamp = self.line_clamp;

                let needs_scroll = line_count > line_clamp;

                let list = uniform_list(
                    self.id.clone(),
                    line_count,
                    move |visible_range, _window, cx| {
                        let state = input_state.read(cx);
                        let value = state.value();
                        let selected_range = state.selected_range.clone();
                        let cursor_offset = state.cursor_offset();
                        let is_empty = value.is_empty();

                        let mut line_offsets: Vec<(usize, usize)> = Vec::new();
                        let mut start = 0;
                        for line in value.split('\n') {
                            let end = start + line.len();
                            line_offsets.push((start, end));
                            start = end + 1;
                        }

                        visible_range
                            .map(|line_idx| {
                                let (line_start, line_end) =
                                    line_offsets.get(line_idx).copied().unwrap_or((0, 0));

                                LineElement {
                                    input: input_state.clone(),
                                    line_index: line_idx,
                                    line_start_offset: line_start,
                                    line_end_offset: line_end,
                                    text_color,
                                    placeholder_text_color,
                                    highlight_text_color,
                                    line_height,
                                    font_size,
                                    font: font.clone(),
                                    transform_text: transform_text.clone(),
                                    cursor_visible,
                                    selected_range: selected_range.clone(),
                                    cursor_offset,
                                    placeholder: placeholder.clone(),
                                    is_empty,
                                }
                            })
                            .collect()
                    },
                )
                .track_scroll(&scroll_handle)
                .map(|mut list| {
                    if !needs_scroll {
                        list.style().overflow.y = Some(Overflow::Hidden);
                    }
                    list
                })
                .h(multiline_height(
                    line_height,
                    line_clamp.min(line_count).max(1),
                    scale_factor,
                ));

                this.child(UniformListInputElement {
                    input: self.state.clone(),
                    child: list.into_any_element(),
                })
            })
            .when(is_multiline && self.word_wrap, |this| {
                let font = font.clone();
                let scroll_handle = self.state.read(cx).scroll_handle.clone();
                let cached_wrap_width = self.state.read(cx).cached_wrap_width;
                let input_state = self.state.clone();
                let transform_text = self.transform_text.clone();
                let placeholder = self.placeholder.clone();
                let line_clamp = self.line_clamp;

                let wrap_width = cached_wrap_width.unwrap_or(px(300.));
                let visual_line_count = self.state.update(cx, |state, _cx| {
                    let should_recompute =
                        state.needs_wrap_recompute || state.precomputed_visual_lines.is_empty();

                    if should_recompute {
                        state.needs_wrap_recompute = false;
                        let count = state.precompute_wrapped_lines(
                            wrap_width,
                            font_size,
                            font.clone(),
                            text_color,
                            window,
                        );
                        state.is_wrapped = true;
                        count
                    } else {
                        state.is_wrapped = true;
                        // Handle deferred scroll even when not recomputing wrapped lines
                        if state.scroll_to_cursor_on_next_render {
                            state.scroll_to_cursor_on_next_render = false;
                            state.ensure_cursor_visible();
                        }
                        state.precomputed_visual_lines.len()
                    }
                });

                let needs_scroll = visual_line_count > line_clamp;

                let list = uniform_list(
                    self.id.clone(),
                    visual_line_count,
                    move |visible_range, _window, cx| {
                        let state = input_state.read(cx);
                        let selected_range = state.selected_range.clone();
                        let cursor_offset = state.cursor_offset();
                        let is_empty = state.value().is_empty();

                        visible_range
                            .map(|visual_idx| WrappedLineElement {
                                input: input_state.clone(),
                                visual_line_index: visual_idx,
                                text_color,
                                placeholder_text_color,
                                highlight_text_color,
                                line_height,
                                font_size,
                                font: font.clone(),
                                transform_text: transform_text.clone(),
                                cursor_visible,
                                selected_range: selected_range.clone(),
                                cursor_offset,
                                placeholder: placeholder.clone(),
                                is_empty,
                            })
                            .collect()
                    },
                )
                .track_scroll(&scroll_handle)
                .map(|mut list| {
                    if !needs_scroll {
                        list.style().overflow.y = Some(Overflow::Hidden);
                    }
                    list
                })
                .h(multiline_height(
                    line_height,
                    line_clamp.min(visual_line_count).max(1),
                    scale_factor,
                ));

                this.child(UniformListInputElement {
                    input: self.state.clone(),
                    child: list.into_any_element(),
                })
            })
            .when(!is_multiline, |this| {
                this.child(TextElement {
                    input: self.state.clone(),
                    placeholder: self.placeholder,
                    text_color,
                    placeholder_text_color,
                    highlight_text_color,
                    line_height,
                    font_size,
                    font,
                    transform_text: self.transform_text,
                    cursor_visible,
                })
            })
    }
}

/// Registers default key bindings for text input. Call once at app startup.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        // Basic navigation (universal)
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("up", Up, None),
        KeyBinding::new("down", Down, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
        KeyBinding::new("enter", InsertNewline, None),
        KeyBinding::new("shift-enter", InsertNewlineShift, None),
        // Basic selection (universal)
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("shift-up", SelectUp, None),
        KeyBinding::new("shift-down", SelectDown, None),
        KeyBinding::new("shift-home", SelectToStartOfLine, None),
        KeyBinding::new("shift-end", SelectToEndOfLine, None),
        // Clipboard & Undo (macOS: cmd, other: ctrl)
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAll, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-x", Cut, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-v", Paste, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-z", Undo, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-z", Redo, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-x", Cut, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-v", Paste, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-z", Undo, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-y", Redo, None),
        // Word navigation (macOS: alt, other: ctrl)
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-left", MoveToPreviousWord, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-right", MoveToNextWord, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-shift-left", SelectToPreviousWordStart, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-shift-right", SelectToNextWordEnd, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-left", MoveToPreviousWord, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-right", MoveToNextWord, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-left", SelectToPreviousWordStart, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-shift-right", SelectToNextWordEnd, None),
        // Word deletion (macOS: alt, other: ctrl)
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-backspace", DeleteToPreviousWordStart, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-delete", DeleteToNextWordEnd, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-backspace", DeleteToPreviousWordStart, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-delete", DeleteToNextWordEnd, None),
        // Line navigation (macOS only)
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-left", MoveToStartOfLine, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-right", MoveToEndOfLine, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-a", MoveToStartOfLine, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-e", MoveToEndOfLine, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-left", SelectToStartOfLine, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-right", SelectToEndOfLine, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-shift-a", SelectToStartOfLine, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-shift-e", SelectToEndOfLine, None),
        // Document navigation (macOS only)
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-up", MoveToStart, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-down", MoveToEnd, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-up", SelectToStart, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-down", SelectToEnd, None),
        // Line deletion (macOS only)
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-backspace", DeleteToBeginningOfLine, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-delete", DeleteToEndOfLine, None),
        // Character palette (macOS only)
        #[cfg(target_os = "macos")]
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, None),
    ]);

    cx.on_keyboard_layout_change(move |cx| {
        for window in cx.windows() {
            window
                .update(cx, |this, _, cx| cx.notify(this.entity_id()))
                .ok();
        }
    })
    .detach();
}

impl Focusable for Input {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus_handle.clone()
    }
}
