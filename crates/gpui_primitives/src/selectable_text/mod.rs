//! Selectable text component for displaying read-only text with selection support.

mod elements;
mod state;

use gpui::{
    AbsoluteLength, App, CursorStyle, ElementId, Entity, FocusHandle, Focusable, Font, Hsla,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, Overflow, ParentElement, Refineable,
    RenderOnce, SharedString, StyleRefinement, Styled, Window, div, prelude::FluentBuilder, px,
    rgb, uniform_list,
};

use crate::utils::{
    TextNavigation, WRAP_WIDTH_EPSILON, multiline_height, pixel_perfect_round, rgb_a,
};
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
        self.state.read(cx).get_text()
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

        let text_color = self
            .style
            .text
            .color
            .unwrap_or_else(|| rgb(0xE8E4FF).into());

        // Pre-measure text width during render phase (before layout) for w_auto support
        // This ensures we have the measured width available when request_layout runs
        if self.state.read(cx).measured_max_line_width.is_none() {
            let text = self.state.read(cx).get_text();
            if !text.is_empty() {
                let mut max_width = gpui::px(0.);
                for line_text in text.split('\n') {
                    let run = gpui::TextRun {
                        len: line_text.len(),
                        font: font.clone(),
                        color: text_color,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    };
                    let shaped = window.text_system().shape_line(
                        line_text.to_string().into(),
                        font_size,
                        &[run],
                        None,
                    );
                    if shaped.width > max_width {
                        max_width = shaped.width;
                    }
                }
                self.state.update(cx, |state, _cx| {
                    state.measured_max_line_width = Some(max_width);
                });
            }
        }

        let state = self.state.read(cx);

        let highlight_text_color = self
            .selection_color
            .unwrap_or_else(|| rgb_a(0x488BFF, 0.3).into());

        // Get measured width and cached wrap width for w_auto support on the outer container
        let container_width = state.measured_max_line_width;
        let cached_wrap_width = state.cached_wrap_width;
        // Check if user specified w_auto (width is None or Length::Auto)
        let user_wants_auto_width = match self.style.size.width {
            None => true,                     // No width set, treat as auto
            Some(gpui::Length::Auto) => true, // Explicitly set to auto
            Some(_) => false,                 // Explicit width set (px, %, etc.)
        };

        // Extract absolute max_width if set (for pre-clamping in code).
        // For relative max_width (percentages), we can't convert to pixels during render,
        // but GPUI's layout will still clamp via max_size (works like CSS).
        let max_width_px: Option<gpui::Pixels> = match self.style.max_size.width {
            Some(gpui::Length::Definite(gpui::DefiniteLength::Absolute(abs))) => {
                Some(abs.to_pixels(window.rem_size()))
            }
            _ => None,
        };

        // Check if any max_width is set (absolute or relative)
        let has_max_width_constraint = self.style.max_size.width.is_some();

        div()
            .id(self.id.clone())
            .min_w_0() // Allow shrinking in flex contexts
            .map(|mut this| {
                this.style().refine(&self.style);
                // If user wants auto width, set width based on content or cached wrap width
                if user_wants_auto_width {
                    if has_max_width_constraint {
                        // With max-width constraint (like max_w_full):
                        // - If we have cached width (from prepaint detecting actual available space),
                        //   compare with measured to decide between auto and fill
                        // - If no cached width yet (first render), use relative(1.) so GPUI's
                        //   max_size constraint can clamp us, then prepaint will cache the result
                        match (cached_wrap_width, container_width) {
                            (Some(cached), Some(measured)) => {
                                let auto_width = measured + WRAP_WIDTH_EPSILON;
                                if auto_width <= cached {
                                    // Text fits within available space - use auto width
                                    this.style().size.width = Some(auto_width.into());
                                } else {
                                    // Text exceeds available space - fill parent, max_w will clamp
                                    this.style().size.width = Some(gpui::relative(1.).into());
                                }
                            }
                            (Some(cached), None) => {
                                // No measured width yet - use cached as fallback
                                this.style().size.width = Some(cached.into());
                            }
                            (None, Some(measured)) => {
                                // First render without cached width: use measured width.
                                // Using relative(1.) doesn't work with right-aligned containers
                                // because the parent doesn't provide a constrained width.
                                let auto_width = measured + WRAP_WIDTH_EPSILON;
                                this.style().size.width = Some(auto_width.into());
                            }
                            (None, None) => {
                                // No width info at all: use relative(1.) as last resort
                                this.style().size.width = Some(gpui::relative(1.).into());
                            }
                        }
                    } else {
                        // No max-width: use measured width, clamped by absolute max if present
                        if let Some(measured) = container_width {
                            let auto_width = measured + WRAP_WIDTH_EPSILON;
                            let width = match max_width_px {
                                Some(max_w) => auto_width.min(max_w),
                                None => auto_width,
                            };
                            this.style().size.width = Some(width.into());
                        }
                    }
                }
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
                // Get measured width for w_auto support (only use when user wants auto width)
                let measured_width = if user_wants_auto_width {
                    state.measured_max_line_width
                } else {
                    None // Fixed width - let layout handle it
                };

                let needs_scroll = line_count > line_clamp;

                let list = uniform_list(
                    self.id.clone(),
                    line_count,
                    move |visible_range, _window, cx| {
                        let state = state_entity.read(cx);
                        let value = state.get_text();
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
                                    measured_width,
                                }
                            })
                            .collect()
                    },
                )
                .track_scroll(&scroll_handle)
                .map(move |mut list| {
                    if !needs_scroll {
                        list.style().overflow.y = Some(Overflow::Hidden);
                    }
                    // Apply measured width for w_auto support, clamped to max_width
                    // GPUI's layout will clamp via max_size if set (works like CSS)
                    if let Some(width) = measured_width {
                        let auto_width = width + WRAP_WIDTH_EPSILON;
                        let clamped = match max_width_px {
                            Some(max_w) => auto_width.min(max_w),
                            None => auto_width,
                        };
                        list.style().size.width = Some(clamped.into());
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
                let measured_max_line_width = self.state.read(cx).measured_max_line_width;
                let state_entity = self.state.clone();
                let line_clamp = self.line_clamp;

                // Calculate effective width for w_auto support:
                // - Always set size.width to measured text width
                // - GPUI's max_size will clamp for relative max widths (like max_w_full)
                // - The elements.rs prepaint will detect if we were clamped and update cached_wrap_width
                let wrap_width = cached_wrap_width
                    .or(measured_max_line_width)
                    .unwrap_or(px(300.));
                // Apply absolute max_width constraint to wrap_width if available
                let wrap_width = match max_width_px {
                    Some(max_w) => wrap_width.min(max_w),
                    None => wrap_width,
                };
                // Calculate effective width for w_auto support.
                // Same logic as outer div: use cached width to decide between auto and fill.
                let (effective_width, use_relative_width) = if user_wants_auto_width {
                    if has_max_width_constraint {
                        // With max-width constraint (like max_w_full):
                        // - If we have cached width, compare with measured to decide
                        // - If no cached width yet (first render), use relative(1.)
                        match (cached_wrap_width, measured_max_line_width) {
                            (Some(cached), Some(measured)) => {
                                let auto_width = measured + WRAP_WIDTH_EPSILON;
                                if auto_width <= cached {
                                    // Text fits within available space - use auto width
                                    (Some(auto_width), false)
                                } else {
                                    // Text exceeds available space - fill parent
                                    (None, true)
                                }
                            }
                            (Some(cached), None) => {
                                // No measured width yet - use cached as fallback
                                (Some(cached), false)
                            }
                            (None, Some(measured)) => {
                                // First render without cached width: use measured width.
                                // Using relative(1.) doesn't work with right-aligned containers
                                // because the parent doesn't provide a constrained width.
                                let auto_width = measured + WRAP_WIDTH_EPSILON;
                                (Some(auto_width), false)
                            }
                            (None, None) => {
                                // No width info at all: use relative(1.) as last resort
                                (None, true)
                            }
                        }
                    } else {
                        // No max-width: always use measured width
                        match measured_max_line_width {
                            Some(measured) => {
                                let auto_width = measured + WRAP_WIDTH_EPSILON;
                                let clamped = match max_width_px {
                                    Some(max_w) => auto_width.min(max_w),
                                    None => auto_width,
                                };
                                (Some(clamped), false)
                            }
                            None => (None, false),
                        }
                    }
                } else {
                    (None, false) // Fixed width - let layout handle it
                };

                let visual_line_count = self.state.update(cx, |state, _cx| {
                    // Track whether we're using auto width so prepaint knows how to interpret changes
                    state.using_auto_width = !use_relative_width && effective_width.is_some();

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
                .map(move |mut list| {
                    if !needs_scroll {
                        list.style().overflow.y = Some(Overflow::Hidden);
                    }
                    // Always use relative width so element can shrink with container.
                    // This ensures the uniform_list receives actual container bounds
                    // during prepaint, enabling proper width-change detection.
                    list.style().size.width = Some(gpui::relative(1.).into());
                    // Use max-width to limit expansion in auto-width mode.
                    // This allows the element to shrink below this width when container shrinks.
                    if let Some(width) = effective_width {
                        list.style().max_size.width = Some(width.into());
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

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::Pixels;

    /// Helper to compute wrap_width given the inputs, mimicking the logic in render()
    fn compute_wrap_width(
        cached_wrap_width: Option<Pixels>,
        measured_max_line_width: Option<Pixels>,
        max_width_px: Option<Pixels>,
    ) -> Pixels {
        let wrap_width = cached_wrap_width
            .or(measured_max_line_width)
            .unwrap_or(px(300.));
        match max_width_px {
            Some(max_w) => wrap_width.min(max_w),
            None => wrap_width,
        }
    }

    /// Helper to compute effective_width and use_relative_width, mimicking the logic in render()
    /// Returns (effective_width, use_relative_width)
    fn compute_effective_width(
        user_wants_auto_width: bool,
        has_max_width_constraint: bool,
        cached_wrap_width: Option<Pixels>,
        measured_max_line_width: Option<Pixels>,
        max_width_px: Option<Pixels>,
    ) -> (Option<Pixels>, bool) {
        if user_wants_auto_width {
            if has_max_width_constraint {
                // With max-width constraint (like max_w_full):
                // - If we have cached width, compare with measured to decide
                // - If no cached width yet (first render), use relative(1.)
                match (cached_wrap_width, measured_max_line_width) {
                    (Some(cached), Some(measured)) => {
                        let auto_width = measured + WRAP_WIDTH_EPSILON;
                        if auto_width <= cached {
                            // Text fits within available space - use auto width
                            (Some(auto_width), false)
                        } else {
                            // Text exceeds available space - fill parent
                            (None, true)
                        }
                    }
                    (Some(cached), None) => {
                        // No measured width yet - use cached as fallback
                        (Some(cached), false)
                    }
                    (None, _) => {
                        // First render: use relative(1.) so max_w_full() can clamp us
                        (None, true)
                    }
                }
            } else {
                // No max-width: always use measured width
                match measured_max_line_width {
                    Some(measured) => {
                        let auto_width = measured + WRAP_WIDTH_EPSILON;
                        let clamped = match max_width_px {
                            Some(max_w) => auto_width.min(max_w),
                            None => auto_width,
                        };
                        (Some(clamped), false)
                    }
                    None => (None, false),
                }
            }
        } else {
            (None, false) // Fixed width - let layout handle it
        }
    }

    /// Helper to check if width change should trigger recompute, mimicking prepaint logic
    fn should_trigger_recompute(
        actual_line_width: Pixels,
        precomputed_width: Pixels,
        needs_wrap_recompute: bool,
    ) -> bool {
        let was_clamped = actual_line_width < precomputed_width - WRAP_WIDTH_EPSILON;
        let width_increased = actual_line_width > precomputed_width + WRAP_WIDTH_EPSILON;
        (was_clamped || width_increased) && !needs_wrap_recompute
    }

    // ==================== wrap_width tests ====================

    #[test]
    fn test_wrap_width_uses_cached_wrap_width_first() {
        let result = compute_wrap_width(Some(px(200.)), Some(px(400.)), None);
        assert_eq!(result, px(200.));
    }

    #[test]
    fn test_wrap_width_falls_back_to_measured_width() {
        let result = compute_wrap_width(None, Some(px(400.)), None);
        assert_eq!(result, px(400.));
    }

    #[test]
    fn test_wrap_width_defaults_to_300_when_nothing_available() {
        let result = compute_wrap_width(None, None, None);
        assert_eq!(result, px(300.));
    }

    #[test]
    fn test_wrap_width_clamped_by_absolute_max_width() {
        // cached_wrap_width is larger than max_width_px - should be clamped
        let result = compute_wrap_width(Some(px(500.)), None, Some(px(300.)));
        assert_eq!(result, px(300.));
    }

    #[test]
    fn test_wrap_width_not_clamped_when_smaller_than_max() {
        // cached_wrap_width is smaller than max_width_px - should not be clamped
        let result = compute_wrap_width(Some(px(200.)), None, Some(px(300.)));
        assert_eq!(result, px(200.));
    }

    #[test]
    fn test_wrap_width_measured_clamped_by_max() {
        // No cached, measured is larger than max - should be clamped
        let result = compute_wrap_width(None, Some(px(500.)), Some(px(300.)));
        assert_eq!(result, px(300.));
    }

    // ==================== effective_width tests ====================

    #[test]
    fn test_effective_width_uses_relative_on_first_render_with_max_constraint() {
        // First render (no cached width) with max_width constraint: use relative(1.)
        // This allows GPUI's max_size to clamp, prepaint will detect and cache actual width
        let (width, use_relative) = compute_effective_width(true, true, None, Some(px(400.)), None);
        assert_eq!(width, None);
        assert!(use_relative);
    }

    #[test]
    fn test_effective_width_uses_auto_when_text_fits() {
        // With cached width and measured width that fits: use auto width
        let (width, use_relative) =
            compute_effective_width(true, true, Some(px(500.)), Some(px(400.)), None);
        assert_eq!(width, Some(px(400.) + WRAP_WIDTH_EPSILON));
        assert!(!use_relative);
    }

    #[test]
    fn test_effective_width_uses_relative_when_text_exceeds_available() {
        // With cached width and measured width that exceeds: use relative(1.)
        let (width, use_relative) =
            compute_effective_width(true, true, Some(px(300.)), Some(px(400.)), None);
        assert_eq!(width, None);
        assert!(use_relative);
    }

    #[test]
    fn test_effective_width_uses_cached_when_no_measured() {
        // With cached width but no measured: use cached as fallback
        let (width, use_relative) = compute_effective_width(true, true, Some(px(300.)), None, None);
        assert_eq!(width, Some(px(300.)));
        assert!(!use_relative);
    }

    #[test]
    fn test_effective_width_uses_relative_when_nothing_available() {
        // No cached, no measured: use relative to fill parent
        let (width, use_relative) = compute_effective_width(true, true, None, None, None);
        assert_eq!(width, None);
        assert!(use_relative);
    }

    #[test]
    fn test_effective_width_uses_measured_when_no_max_constraint() {
        // Without max_width constraint, should use measured width + epsilon
        let (width, use_relative) =
            compute_effective_width(true, false, None, Some(px(400.)), None);
        assert_eq!(width, Some(px(400.) + WRAP_WIDTH_EPSILON));
        assert!(!use_relative);
    }

    #[test]
    fn test_effective_width_clamped_by_absolute_max_when_no_relative_constraint() {
        // Without relative max_width constraint but with absolute max_width_px
        let (width, use_relative) =
            compute_effective_width(true, false, None, Some(px(400.)), Some(px(300.)));
        assert_eq!(width, Some(px(300.)));
        assert!(!use_relative);
    }

    #[test]
    fn test_effective_width_none_when_not_auto_width() {
        // When user doesn't want auto width (fixed width), effective_width should be None
        let (width, use_relative) =
            compute_effective_width(false, false, None, Some(px(400.)), None);
        assert_eq!(width, None);
        assert!(!use_relative);
    }

    #[test]
    fn test_effective_width_none_when_not_measured_yet_no_max_constraint() {
        // When text hasn't been measured yet (no max constraint), effective_width should be None
        let (width, use_relative) = compute_effective_width(true, false, None, None, None);
        assert_eq!(width, None);
        assert!(!use_relative);
    }

    // ==================== recompute trigger tests ====================

    #[test]
    fn test_recompute_triggered_when_clamped() {
        // actual < precomputed - WRAP_WIDTH_EPSILON means we were clamped
        let result = should_trigger_recompute(px(200.), px(400.), false);
        assert!(result, "Should trigger recompute when width was clamped");
    }

    #[test]
    fn test_recompute_triggered_when_width_increased() {
        // actual > precomputed + WRAP_WIDTH_EPSILON means width increased
        let result = should_trigger_recompute(px(400.), px(200.), false);
        assert!(result, "Should trigger recompute when width increased");
    }

    #[test]
    fn test_recompute_not_triggered_when_width_within_epsilon() {
        // actual is within epsilon of precomputed - no recompute needed
        let result = should_trigger_recompute(px(200.), px(200.5), false);
        assert!(
            !result,
            "Should not trigger recompute when width is within epsilon"
        );
    }

    #[test]
    fn test_recompute_not_triggered_when_already_pending() {
        // Even if width changed, don't trigger if recompute is already pending
        let result = should_trigger_recompute(px(200.), px(400.), true);
        assert!(
            !result,
            "Should not trigger recompute when one is already pending"
        );
    }

    #[test]
    fn test_recompute_not_triggered_for_small_decrease() {
        // Width decreased but within epsilon threshold
        let precomputed = px(200.);
        let actual = precomputed - px(1.0); // Less than WRAP_WIDTH_EPSILON (1.5)
        let result = should_trigger_recompute(actual, precomputed, false);
        assert!(
            !result,
            "Should not trigger recompute for decrease within epsilon"
        );
    }

    #[test]
    fn test_recompute_triggered_for_decrease_beyond_epsilon() {
        // Width decreased beyond epsilon threshold
        let precomputed = px(200.);
        let actual = precomputed - px(2.0); // More than WRAP_WIDTH_EPSILON (1.5)
        let result = should_trigger_recompute(actual, precomputed, false);
        assert!(
            result,
            "Should trigger recompute for decrease beyond epsilon"
        );
    }
}
