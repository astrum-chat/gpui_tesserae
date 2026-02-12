mod cursor_blink;
mod elements;
mod selection;
mod state;

/// Text transformation functions for input display (e.g., password masking).
pub mod text_transforms;

use std::sync::Arc;

use gpui::{
    App, CursorStyle, ElementId, Entity, FocusHandle, Focusable, Hsla, InteractiveElement,
    IntoElement, KeyBinding, MouseButton, Overflow, ParentElement, Refineable, RenderOnce,
    SharedString, Style, StyleRefinement, Styled, Window, div, hsla, prelude::FluentBuilder,
    relative, rgb, uniform_list,
};

use crate::input::state::{SecondarySubmit, Submit};
use crate::utils::{TextNavigation, multiline_height, rgb_a};
pub use cursor_blink::CursorBlink;
use elements::{LineElement, TextElement, UniformListInputElement, WrappedTextInputElement};
pub use state::{
    Backspace, Copy, Cut, Delete, DeleteToBeginningOfLine, DeleteToEndOfLine, DeleteToNextWordEnd,
    DeleteToPreviousWordStart, Down, End, Home, InputState, Left, MapTextFn, MoveToEnd,
    MoveToEndOfLine, MoveToNextWord, MoveToPreviousWord, MoveToStart, MoveToStartOfLine, Paste,
    Quit, Redo, Right, SelectAll, SelectDown, SelectLeft, SelectRight, SelectToEnd,
    SelectToEndOfLine, SelectToNextWordEnd, SelectToPreviousWordStart, SelectToStart,
    SelectToStartOfLine, SelectUp, ShowCharacterPalette, Undo, Up, VisibleLineInfo, VisualLineInfo,
};

pub(crate) type TransformTextFn = Arc<dyn Fn(char) -> char + Send + Sync>;
/// Callback function type for handling Enter key presses in the input.
pub type OnEnterFn = Arc<dyn Fn(&mut Window, &mut App) + 'static>;

/// A text input element supporting single-line and multi-line editing with selection, clipboard, and undo/redo.
#[derive(IntoElement)]
pub struct Input {
    id: ElementId,
    state: Entity<InputState>,
    disabled: bool,
    multiline_clamp: Option<usize>,
    multiline_wrapped: bool,
    on_submit: Option<OnEnterFn>,
    submit_disabled: bool,
    secondary_newline: bool,
    placeholder: SharedString,
    placeholder_text_color: Option<Hsla>,
    selection_color: Option<Hsla>,
    selection_rounded: Option<gpui::Pixels>,
    selection_rounded_smoothing: Option<f32>,
    selection_precise: bool,
    transform_text: Option<TransformTextFn>,
    map_text: Option<MapTextFn>,
    debug_interior_corners: bool,
    style: StyleRefinement,
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

#[allow(missing_docs)]
impl Input {
    pub fn new(id: impl Into<ElementId>, state: Entity<InputState>) -> Self {
        Self {
            id: id.into(),
            state,
            disabled: false,
            multiline_clamp: None,
            multiline_wrapped: false,
            on_submit: None,
            submit_disabled: false,
            secondary_newline: false,
            placeholder: "Type here...".into(),
            placeholder_text_color: None,
            selection_color: None,
            selection_rounded: None,
            selection_rounded_smoothing: None,
            selection_precise: false,
            transform_text: None,
            map_text: None,
            debug_interior_corners: false,
            style: StyleRefinement::default(),
        }
    }

    /// Sets the maximum number of visible lines before scrolling. Use `multiline()` for unlimited.
    pub fn multiline_clamp(mut self, multiline_clamp: usize) -> Self {
        self.multiline_clamp = Some(multiline_clamp.max(1));
        self
    }

    pub fn multiline(mut self) -> Self {
        self.multiline_clamp = Some(usize::MAX);
        self
    }

    pub fn multiline_wrapped(mut self) -> Self {
        self.multiline_wrapped = true;
        self
    }

    /// Sets a callback to invoke on `Submit` action.
    /// Forces the `InsertNewlineSecondary` action to be used for newline.
    pub fn on_submit(mut self, callback: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_submit = Some(Arc::new(callback));
        self
    }

    /// Disables the submit action when set to `true`.
    /// When disabled, the `Submit` action will not trigger the `on_submit` callback.
    pub fn submit_disabled(mut self, disabled: bool) -> Self {
        self.submit_disabled = disabled;
        self
    }

    /// Forces the `InsertNewlineSecondary` action to be used for newline.
    pub fn secondary_newline(mut self) -> Self {
        self.secondary_newline = true;
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

    pub fn placeholder_text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.placeholder_text_color = Some(color.into());
        self
    }

    pub fn selection_color(mut self, color: impl Into<Hsla>) -> Self {
        self.selection_color = Some(color.into());
        self
    }

    /// Sets the corner radius for selection highlighting.
    ///
    /// When set to a value greater than 0, selection rectangles will have rounded
    /// corners. For multi-line selections, inner corners (where the selection wraps
    /// to a new line) will also be properly rounded based on adjacent line positions.
    pub fn selection_rounded(mut self, radius: impl Into<gpui::Pixels>) -> Self {
        self.selection_rounded = Some(radius.into());
        self
    }

    /// Sets the corner smoothing for selection highlighting (squircle effect).
    ///
    /// A value of 0.0 uses standard rounded corners (fast path).
    /// A value of 1.0 uses full Figma-style squircle smoothing.
    /// Values in between provide intermediate smoothing.
    ///
    /// This only has an effect when `selection_rounded` is also set to a value > 0.
    /// When smoothing is 0 or unset, the faster PaintQuad rendering path is used.
    pub fn selection_rounded_smoothing(mut self, smoothing: f32) -> Self {
        self.selection_rounded_smoothing = Some(smoothing.clamp(0.0, 1.0));
        self
    }

    /// Uses precise selection highlighting that stops at the last selected character.
    ///
    /// By default, selection extends to the right edge of the container on lines
    /// where the selection continues to the next line. This method disables that
    /// behavior so the highlight exactly wraps the selected text.
    pub fn selection_precise(mut self) -> Self {
        self.selection_precise = true;
        self
    }

    /// Enables debug visualization of interior (concave) selection corners.
    /// When enabled, interior corner patches are painted red instead of the selection color.
    pub fn debug_interior_corners(mut self, enabled: bool) -> Self {
        self.debug_interior_corners = enabled;
        self
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn get_placeholder(&self) -> &SharedString {
        &self.placeholder
    }

    pub fn read_text(&self, cx: &mut App) -> SharedString {
        self.state.read(cx).value()
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_multiline = self.multiline_clamp.is_some_and(|c| c > 1);

        let params = crate::utils::compute_text_render_params(&self.style.text, window);
        let font = params.font;
        let font_size = params.font_size;
        let line_height = params.line_height;
        let scale_factor = params.scale_factor;

        let map_text = self.map_text.clone();
        self.state.update(cx, |state, cx| {
            state.update_focus_state(window, cx);
            state.set_multiline_params(is_multiline, line_height, self.multiline_clamp);
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
            .min_w_0()
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
            .map(|this| {
                register_input_actions(
                    this,
                    window,
                    &self.state,
                    is_multiline,
                    self.on_submit.clone(),
                    self.secondary_newline,
                    self.submit_disabled,
                )
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
            .when(is_multiline && !self.multiline_wrapped, |this| {
                let font = font.clone();
                let line_count = state.line_count().max(1);
                let scroll_handle = state.scroll_handle.clone();
                let input_state = self.state.clone();
                let transform_text = self.transform_text.clone();
                let placeholder = self.placeholder.clone();
                let multiline_clamp = self.multiline_clamp;
                let selection_rounded = self.selection_rounded;
                let selection_rounded_smoothing = self.selection_rounded_smoothing;
                let selection_precise = self.selection_precise;
                let debug_interior_corners = self.debug_interior_corners;

                let needs_scroll = multiline_clamp.map_or(false, |clamp| line_count > clamp);

                let list = uniform_list(
                    self.id.clone(),
                    line_count,
                    move |visible_range, _window, cx| {
                        let state = input_state.read(cx);
                        let value = state.value();
                        let selected_range = state.selected_range.clone();
                        let cursor_offset = state.cursor_offset();

                        let line_offsets = crate::utils::compute_line_offsets(&value);

                        visible_range
                            .map(|line_idx| {
                                let (line_start, line_end) =
                                    line_offsets.get(line_idx).copied().unwrap_or((0, 0));

                                let prev_line_offsets = if line_idx > 0 {
                                    line_offsets.get(line_idx - 1).copied()
                                } else {
                                    None
                                };
                                let next_line_offsets = line_offsets.get(line_idx + 1).copied();

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
                                    selection_rounded,
                                    selection_rounded_smoothing,
                                    prev_line_offsets,
                                    next_line_offsets,
                                    selection_precise,
                                    debug_interior_corners,
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
                    multiline_clamp.map_or(1, |c| c.min(line_count)).max(1),
                    scale_factor,
                ));

                this.child(UniformListInputElement {
                    input: self.state.clone(),
                    child: list.into_any_element(),
                })
            })
            .when(is_multiline && self.multiline_wrapped, |this| {
                let font = font.clone();

                // Cache render params on state (needed for UniformListInputElement's
                // rewrap_at_width fallback). The measure callback handles wrapping
                // with the actual width.
                self.state.update(cx, |state, _cx| {
                    state.last_font = Some(font.clone());
                    state.last_font_size = Some(font_size);
                    state.last_text_color = Some(text_color);
                    state.is_wrapped = true;
                    state.needs_wrap_recompute = false;
                });

                // The parent div keeps the user's sizing (w, max_w, etc.).
                // The WrappedTextInputElement fills the parent at 100% width
                // so it gets the parent's resolved width in its measure callback.
                let mut element_style = Style::default();
                element_style.size.width = relative(1.).into();

                this.child(WrappedTextInputElement {
                    input: self.state.clone(),
                    text_color,
                    placeholder_text_color,
                    highlight_text_color,
                    line_height,
                    font_size,
                    font: font.clone(),
                    transform_text: self.transform_text.clone(),
                    cursor_visible,
                    placeholder: self.placeholder.clone(),
                    selection_rounded: self.selection_rounded,
                    selection_rounded_smoothing: self.selection_rounded_smoothing,
                    selection_precise: self.selection_precise,
                    debug_interior_corners: self.debug_interior_corners,
                    multiline_clamp: self.multiline_clamp,
                    scale_factor,
                    style: element_style,
                    children: Vec::new(),
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
                    selection_rounded: self.selection_rounded,
                    selection_rounded_smoothing: self.selection_rounded_smoothing,
                    selection_precise: self.selection_precise,
                })
            })
    }
}

fn register_input_actions<E>(
    element: E,
    window: &mut Window,
    state: &Entity<InputState>,
    is_multiline: bool,
    on_submit: Option<OnEnterFn>,
    secondary_newline: bool,
    submit_disabled: bool,
) -> E
where
    E: InteractiveElement + FluentBuilder,
{
    element
        .on_action(window.listener_for(state, InputState::backspace))
        .on_action(window.listener_for(state, InputState::delete))
        .on_action(window.listener_for(state, InputState::left))
        .on_action(window.listener_for(state, InputState::right))
        .on_action(window.listener_for(state, InputState::home))
        .on_action(window.listener_for(state, InputState::end))
        .on_action(window.listener_for(state, InputState::select_left))
        .on_action(window.listener_for(state, InputState::select_right))
        .on_action(window.listener_for(state, InputState::select_to_start_of_line))
        .on_action(window.listener_for(state, InputState::select_to_end_of_line))
        .on_action(window.listener_for(state, InputState::select_all))
        .on_action(window.listener_for(state, InputState::copy))
        .on_action(window.listener_for(state, InputState::cut))
        .on_action(window.listener_for(state, InputState::paste))
        .on_action(window.listener_for(state, InputState::undo))
        .on_action(window.listener_for(state, InputState::redo))
        .on_action(window.listener_for(state, InputState::move_to_previous_word))
        .on_action(window.listener_for(state, InputState::move_to_next_word))
        .on_action(window.listener_for(state, InputState::select_to_previous_word_start))
        .on_action(window.listener_for(state, InputState::select_to_next_word_end))
        .on_action(window.listener_for(state, InputState::delete_to_previous_word_start))
        .on_action(window.listener_for(state, InputState::delete_to_next_word_end))
        .on_action(window.listener_for(state, InputState::move_to_start_of_line))
        .on_action(window.listener_for(state, InputState::move_to_end_of_line))
        .on_action(window.listener_for(state, InputState::move_to_start))
        .on_action(window.listener_for(state, InputState::move_to_end))
        .on_action(window.listener_for(state, InputState::select_to_start))
        .on_action(window.listener_for(state, InputState::select_to_end))
        .on_action(window.listener_for(state, InputState::delete_to_beginning_of_line))
        .on_action(window.listener_for(state, InputState::delete_to_end_of_line))
        .on_action(window.listener_for(state, InputState::show_character_palette))
        .when(is_multiline, |this| {
            this.on_action(window.listener_for(state, InputState::up))
                .on_action(window.listener_for(state, InputState::down))
                .on_action(window.listener_for(state, InputState::select_up))
                .on_action(window.listener_for(state, InputState::select_down))
                .map(|this| {
                    let on_submit = on_submit.clone();

                    match (on_submit, secondary_newline) {
                        (None, true) => this.on_action(
                            window.listener_for(state, InputState::insert_newline_secondary),
                        ),

                        (None, false) => {
                            this.on_action(window.listener_for(state, InputState::insert_newline))
                        }

                        (Some(on_submit), _) => this
                            .on_action(
                                window.listener_for(state, InputState::insert_newline_secondary),
                            )
                            .when(!submit_disabled, |this| {
                                this.on_action(move |_: &Submit, window, cx| {
                                    on_submit(window, cx);
                                })
                            }),
                    }
                })
        })
}

/// Registers default key bindings for text input. Call once at app startup.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("up", Up, None),
        KeyBinding::new("down", Down, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
        KeyBinding::new("enter", Submit, None),
        KeyBinding::new("shift-enter", SecondarySubmit, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("shift-up", SelectUp, None),
        KeyBinding::new("shift-down", SelectDown, None),
        KeyBinding::new("shift-home", SelectToStartOfLine, None),
        KeyBinding::new("shift-end", SelectToEndOfLine, None),
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
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-backspace", DeleteToPreviousWordStart, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("alt-delete", DeleteToNextWordEnd, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-backspace", DeleteToPreviousWordStart, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-delete", DeleteToNextWordEnd, None),
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
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-up", MoveToStart, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-down", MoveToEnd, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-up", SelectToStart, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-down", SelectToEnd, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-backspace", DeleteToBeginningOfLine, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-delete", DeleteToEndOfLine, None),
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

#[cfg(all(test, feature = "test-support"))]
mod builder_tests {
    use super::*;
    use gpui::{AppContext as _, TestAppContext, px};

    #[gpui::test]
    fn test_selection_precise_default_false(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));
        cx.update(|_cx| {
            let input = Input::new("test", state);
            assert!(
                !input.selection_precise,
                "selection_precise should default to false"
            );
        });
    }

    #[gpui::test]
    fn test_selection_precise_setter(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));
        cx.update(|_cx| {
            let input = Input::new("test", state).selection_precise();
            assert!(
                input.selection_precise,
                "selection_precise should be true after calling .selection_precise()"
            );
        });
    }

    #[gpui::test]
    fn test_selection_precise_in_builder_chain(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));
        cx.update(|_cx| {
            let input = Input::new("test", state)
                .selection_color(gpui::hsla(0.6, 1., 0.5, 0.3))
                .selection_rounded(px(4.))
                .selection_precise()
                .placeholder("Enter text...");
            assert!(input.selection_precise);
            assert!(input.selection_color.is_some());
            assert!(input.selection_rounded.is_some());
        });
    }

    #[gpui::test]
    fn test_is_selecting_default_false(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));
        state.read_with(cx, |state, _| {
            assert!(!state.is_selecting, "is_selecting should default to false");
        });
    }

    #[gpui::test]
    fn test_is_selecting_set_on_mouse_down_cleared_on_mouse_up(cx: &mut TestAppContext) {
        let state = cx.new(|cx| InputState::new(cx));

        // Simulate mouse down — is_selecting should become true
        state.update(cx, |state, _cx| {
            state.is_selecting = true;
        });
        state.read_with(cx, |state, _| {
            assert!(
                state.is_selecting,
                "is_selecting should be true after mouse down"
            );
        });

        // Simulate mouse up — is_selecting should become false
        state.update(cx, |state, _cx| {
            state.is_selecting = false;
        });
        state.read_with(cx, |state, _| {
            assert!(
                !state.is_selecting,
                "is_selecting should be false after mouse up"
            );
        });
    }
}
