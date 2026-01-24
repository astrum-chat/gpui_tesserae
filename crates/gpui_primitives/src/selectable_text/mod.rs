//! Selectable text component for displaying read-only text with selection support.

mod elements;
mod state;

use gpui::{
    AbsoluteLength, App, CursorStyle, ElementId, Entity, FocusHandle, Focusable, Font, Hsla,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, Overflow, ParentElement, Refineable,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
    rgb, uniform_list,
};

use crate::utils::{TextNavigation, multiline_height, pixel_perfect_round, rgb_a};
use elements::{LineElement, UniformListElement, WrappedLineElement};

pub use state::{
    Copy, Down, End, Home, Left, MoveToEnd, MoveToEndOfLine, MoveToNextWord, MoveToPreviousWord,
    MoveToStart, MoveToStartOfLine, Right, SelectAll, SelectDown, SelectLeft, SelectRight,
    SelectToEnd, SelectToEndOfLine, SelectToNextWordEnd, SelectToPreviousWordStart, SelectToStart,
    SelectToStartOfLine, SelectUp, SelectableTextState, Up, VisibleLineInfo, VisualLineInfo,
};

/// A selectable text element for displaying read-only text with selection and copy support.
/// Unlike Input, this only supports multiline mode and does not allow editing.
#[derive(IntoElement)]
pub struct SelectableText {
    id: ElementId,
    state: Entity<SelectableTextState>,
    line_clamp: usize,
    word_wrap: bool,
    selection_color: Option<Hsla>,
    style: StyleRefinement,
}

impl Styled for SelectableText {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl SelectableText {
    /// Creates a new selectable text element with the given ID and state entity.
    pub fn new(id: impl Into<ElementId>, state: Entity<SelectableTextState>) -> Self {
        Self {
            id: id.into(),
            state,
            line_clamp: usize::MAX,
            word_wrap: true,
            selection_color: None,
            style: StyleRefinement::default(),
        }
    }

    /// Sets the maximum number of visible lines before scrolling.
    pub fn line_clamp(mut self, line_clamp: usize) -> Self {
        self.line_clamp = line_clamp.max(1);
        self
    }

    /// Enables or disables word wrapping. Default is true.
    pub fn word_wrap(mut self, enabled: bool) -> Self {
        self.word_wrap = enabled;
        self
    }

    /// Sets the background color for selected text.
    pub fn selection_color(mut self, color: impl Into<Hsla>) -> Self {
        self.selection_color = Some(color.into());
        self
    }

    /// Returns the current text value from state.
    pub fn read_text(&self, cx: &mut App) -> SharedString {
        self.state.read(cx).text()
    }
}

impl RenderOnce for SelectableText {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_style = &self.style.text;
        let font_size = match text_style
            .font_size
            .unwrap_or_else(|| window.text_style().font_size)
        {
            AbsoluteLength::Pixels(px) => px,
            AbsoluteLength::Rems(rems) => rems.to_pixels(window.rem_size()),
        };
        let line_height = text_style
            .line_height
            .map(|this| this.to_pixels(font_size.into(), window.rem_size()))
            .unwrap_or_else(|| window.line_height());
        let scale_factor = window.scale_factor();
        let line_height = pixel_perfect_round(line_height, scale_factor);

        let font = Font {
            family: text_style
                .font_family
                .clone()
                .unwrap_or_else(|| window.text_style().font_family),
            features: text_style.font_features.clone().unwrap_or_default(),
            fallbacks: text_style.font_fallbacks.clone(),
            weight: text_style.font_weight.unwrap_or_default(),
            style: text_style.font_style.unwrap_or_default(),
        };

        self.state.update(cx, |state, _cx| {
            state.set_multiline_params(line_height, self.line_clamp);
            state.is_wrapped = self.word_wrap;
        });

        let state = self.state.read(cx);

        let text_color = self
            .style
            .text
            .color
            .unwrap_or_else(|| rgb(0xE8E4FF).into());

        let highlight_text_color = self
            .selection_color
            .unwrap_or_else(|| rgb_a(0x488BFF, 0.3).into());

        div()
            .id(self.id.clone())
            .map(|mut this| {
                this.style().refine(&self.style);
                this
            })
            .key_context("SelectableText")
            .track_focus(&state.focus_handle)
            .cursor(CursorStyle::IBeam)
            // Navigation actions
            .on_action(window.listener_for(&self.state, SelectableTextState::left))
            .on_action(window.listener_for(&self.state, SelectableTextState::right))
            .on_action(window.listener_for(&self.state, SelectableTextState::up))
            .on_action(window.listener_for(&self.state, SelectableTextState::down))
            .on_action(window.listener_for(&self.state, SelectableTextState::home))
            .on_action(window.listener_for(&self.state, SelectableTextState::end))
            // Selection actions
            .on_action(window.listener_for(&self.state, SelectableTextState::select_left))
            .on_action(window.listener_for(&self.state, SelectableTextState::select_right))
            .on_action(window.listener_for(&self.state, SelectableTextState::select_up))
            .on_action(window.listener_for(&self.state, SelectableTextState::select_down))
            .on_action(window.listener_for(&self.state, SelectableTextState::select_all))
            // Line navigation
            .on_action(window.listener_for(&self.state, SelectableTextState::move_to_start_of_line))
            .on_action(window.listener_for(&self.state, SelectableTextState::move_to_end_of_line))
            .on_action(
                window.listener_for(&self.state, SelectableTextState::select_to_start_of_line),
            )
            .on_action(window.listener_for(&self.state, SelectableTextState::select_to_end_of_line))
            // Document navigation
            .on_action(window.listener_for(&self.state, SelectableTextState::move_to_start))
            .on_action(window.listener_for(&self.state, SelectableTextState::move_to_end))
            .on_action(window.listener_for(&self.state, SelectableTextState::select_to_start))
            .on_action(window.listener_for(&self.state, SelectableTextState::select_to_end))
            // Word navigation
            .on_action(window.listener_for(&self.state, SelectableTextState::move_to_previous_word))
            .on_action(window.listener_for(&self.state, SelectableTextState::move_to_next_word))
            .on_action(window.listener_for(
                &self.state,
                SelectableTextState::select_to_previous_word_start,
            ))
            .on_action(
                window.listener_for(&self.state, SelectableTextState::select_to_next_word_end),
            )
            // Copy
            .on_action(window.listener_for(&self.state, SelectableTextState::copy))
            // Mouse handling
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&self.state, SelectableTextState::on_mouse_down),
            )
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(&self.state, SelectableTextState::on_mouse_up),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                window.listener_for(&self.state, SelectableTextState::on_mouse_up),
            )
            .on_mouse_move(window.listener_for(&self.state, SelectableTextState::on_mouse_move))
            .when(!self.word_wrap, |this| {
                // Non-wrapped mode: one line element per logical line
                let font = font.clone();
                let line_count = state.line_count().max(1);
                let scroll_handle = state.scroll_handle.clone();
                let state_entity = self.state.clone();
                let line_clamp = self.line_clamp;

                let needs_scroll = line_count > line_clamp;

                let list = uniform_list(
                    self.id.clone(),
                    line_count,
                    move |visible_range, _window, cx| {
                        let state = state_entity.read(cx);
                        let value = state.text();
                        let selected_range = state.selected_range.clone();
                        let is_select_all = state.is_select_all;

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
                                    state: state_entity.clone(),
                                    line_index: line_idx,
                                    line_start_offset: line_start,
                                    line_end_offset: line_end,
                                    text_color,
                                    highlight_text_color,
                                    line_height,
                                    font_size,
                                    font: font.clone(),
                                    selected_range: selected_range.clone(),
                                    is_select_all,
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

                this.child(UniformListElement {
                    state: self.state.clone(),
                    child: list.into_any_element(),
                })
            })
            .when(self.word_wrap, |this| {
                // Wrapped mode: one element per visual line
                let font = font.clone();
                let scroll_handle = self.state.read(cx).scroll_handle.clone();
                let cached_wrap_width = self.state.read(cx).cached_wrap_width;
                let state_entity = self.state.clone();
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
                        count
                    } else {
                        // Handle deferred scroll even when not recomputing
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
                        let state = state_entity.read(cx);
                        let selected_range = state.selected_range.clone();
                        let is_select_all = state.is_select_all;

                        visible_range
                            .map(|visual_idx| WrappedLineElement {
                                state: state_entity.clone(),
                                visual_line_index: visual_idx,
                                text_color,
                                highlight_text_color,
                                line_height,
                                font_size,
                                font: font.clone(),
                                selected_range: selected_range.clone(),
                                is_select_all,
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

                this.child(UniformListElement {
                    state: self.state.clone(),
                    child: list.into_any_element(),
                })
            })
    }
}

/// Registers default key bindings for selectable text. Call once at app startup.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        // Navigation
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("up", Up, None),
        KeyBinding::new("down", Down, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
        // Selection
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("shift-up", SelectUp, None),
        KeyBinding::new("shift-down", SelectDown, None),
        KeyBinding::new("shift-home", SelectToStartOfLine, None),
        KeyBinding::new("shift-end", SelectToEndOfLine, None),
        // Select all & Copy (macOS: cmd, other: ctrl)
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-a", SelectAll, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-c", Copy, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, None),
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
        // Document navigation (macOS only)
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-up", MoveToStart, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-down", MoveToEnd, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-up", SelectToStart, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-shift-down", SelectToEnd, None),
    ]);
}

impl Focusable for SelectableText {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state.read(cx).focus_handle.clone()
    }
}
