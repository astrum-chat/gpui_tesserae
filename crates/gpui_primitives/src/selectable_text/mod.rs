//! Selectable text component for displaying read-only text with selection support.

mod elements;
mod state;

use gpui::{
    AbsoluteLength, App, CursorStyle, ElementId, Entity, FocusHandle, Focusable, Font, Hsla,
    InteractiveElement, IntoElement, KeyBinding, MouseButton, Overflow, ParentElement, Pixels,
    Refineable, RenderOnce, SharedString, StyleRefinement, Styled, Window, div,
    prelude::FluentBuilder, rgb, uniform_list,
};

use crate::extensions::WindowExt;
use crate::utils::{TextNavigation, WIDTH_WRAP_BASE_MARGIN, multiline_height, rgb_a};
use elements::{LineElement, UniformListElement, WrappedLineElement};

pub use state::{
    Copy, Down, End, Home, Left, MoveToEnd, MoveToEndOfLine, MoveToNextWord, MoveToPreviousWord,
    MoveToStart, MoveToStartOfLine, Right, SelectAll, SelectDown, SelectLeft, SelectRight,
    SelectToEnd, SelectToEndOfLine, SelectToNextWordEnd, SelectToPreviousWordStart, SelectToStart,
    SelectToStartOfLine, SelectUp, SelectableTextState, Up, VisibleLineInfo, VisualLineInfo,
};

fn compute_effective_width(
    user_wants_auto_width: bool,
    has_max_width_constraint: bool,
    cached_wrap_width: Option<Pixels>,
    measured_width: Option<Pixels>,
    max_width_px: Option<Pixels>,
) -> (Option<Pixels>, bool) {
    // Always use just the base margin for div sizing. The whitespace margin
    // is only relevant for compute_wrap_width's fallback estimates — adding
    // it to the div width creates a feedback loop where the div width changes
    // based on the visual line count, causing oscillation.
    let margin = WIDTH_WRAP_BASE_MARGIN;

    if !user_wants_auto_width {
        return (None, false);
    }

    if has_max_width_constraint {
        match (cached_wrap_width, measured_width) {
            (Some(cached), Some(measured)) => {
                let auto_width = measured + margin;
                if auto_width <= cached {
                    (Some(auto_width), false)
                } else {
                    (None, true)
                }
            }
            (Some(cached), None) => (Some(cached), false),
            (None, _) => (None, true),
        }
    } else {
        match measured_width {
            Some(measured) => {
                let auto_width = measured + margin;
                let clamped = max_width_px.map_or(auto_width, |max_w| auto_width.min(max_w));
                (Some(clamped), false)
            }
            None => (None, false),
        }
    }
}

fn compute_wrap_width(
    cached_wrap_width: Option<Pixels>,
    measured_width: Option<Pixels>,
    max_width_px: Option<Pixels>,
    user_wants_auto_width: bool,
) -> Pixels {
    // When we have the actual container width from a previous prepaint (cached_wrap_width),
    // use it directly — it's already the precise available width.
    //
    // Only add base margin when falling back to estimates (measured_width, max_width_px) to
    // avoid premature wrapping from imprecise values.
    if user_wants_auto_width {
        if let Some(cached) = cached_wrap_width {
            return max_width_px.map_or(cached, |max_w| cached.min(max_w));
        }
        let width = max_width_px.or(measured_width).unwrap_or(Pixels::MAX);
        let clamped = max_width_px.map_or(width, |max_w| width.min(max_w));
        clamped + WIDTH_WRAP_BASE_MARGIN
    } else {
        if let Some(cached) = cached_wrap_width {
            return max_width_px.map_or(cached, |max_w| cached.min(max_w));
        }
        let width = max_width_px.unwrap_or(Pixels::MAX);
        let clamped = max_width_px.map_or(width, |max_w| width.min(max_w));
        clamped + WIDTH_WRAP_BASE_MARGIN
    }
}

/// A selectable text element for displaying read-only text with selection and copy support.
#[derive(IntoElement)]
pub struct SelectableText {
    id: ElementId,
    state: Entity<SelectableTextState>,
    line_clamp: usize,
    word_wrap: bool,
    selection_color: Option<Hsla>,
    selection_rounded: Option<Pixels>,
    selection_rounded_smoothing: Option<f32>,
    debug_wrapping: bool,
    debug_character_bounds: bool,
    style: StyleRefinement,
}

impl Styled for SelectableText {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl SelectableText {
    /// Creates a new selectable text element.
    pub fn new(id: impl Into<ElementId>, state: Entity<SelectableTextState>) -> Self {
        Self {
            id: id.into(),
            state,
            line_clamp: usize::MAX,
            word_wrap: true,
            selection_color: None,
            selection_rounded: None,
            selection_rounded_smoothing: None,
            debug_wrapping: false,
            debug_character_bounds: false,
            style: StyleRefinement::default(),
        }
    }

    /// Sets the maximum number of visible lines before scrolling.
    pub fn line_clamp(mut self, line_clamp: usize) -> Self {
        self.line_clamp = line_clamp.max(1);
        self
    }

    /// Enables or disables word wrapping.
    pub fn word_wrap(mut self, enabled: bool) -> Self {
        self.word_wrap = enabled;
        self
    }

    /// Sets the background color for selected text.
    pub fn selection_color(mut self, color: impl Into<Hsla>) -> Self {
        self.selection_color = Some(color.into());
        self
    }

    /// Sets the corner radius for selection highlighting.
    ///
    /// When set to a value greater than 0, selection rectangles will have rounded
    /// corners. For multi-line selections, inner corners (where the selection wraps
    /// to a new line) will also be properly rounded based on adjacent line positions.
    pub fn selection_rounded(mut self, radius: impl Into<Pixels>) -> Self {
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

    /// Enables debug visualization of text wrapping width.
    pub fn debug_wrapping(mut self, enabled: bool) -> Self {
        self.debug_wrapping = enabled;
        self
    }

    /// Enables debug visualization of individual character bounds.
    pub fn debug_character_bounds(mut self, enabled: bool) -> Self {
        self.debug_character_bounds = enabled;
        self
    }

    /// Returns the current text value from state.
    pub fn read_text(&self, cx: &mut App) -> SharedString {
        self.state.read(cx).get_text()
    }

    fn measure_text_width(
        &self,
        font: &Font,
        font_size: Pixels,
        text_color: Hsla,
        window: &mut Window,
        cx: &mut App,
    ) {
        if self.state.read(cx).measured_max_line_width.is_some() {
            return;
        }

        let text = self.state.read(cx).get_text();
        if text.is_empty() {
            return;
        }

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

        let max_width = window.round(max_width);
        self.state.update(cx, |state, _cx| {
            state.measured_max_line_width = Some(max_width);
        });
    }

    fn apply_auto_width(&self, style: &mut StyleRefinement, params: &WidthParams) {
        if !params.user_wants_auto_width {
            return;
        }

        let Some(measured) = params.container_width else {
            return;
        };

        // Always use just the base margin for div sizing — see compute_effective_width.
        let margin = WIDTH_WRAP_BASE_MARGIN;
        let auto_width = measured + margin;

        if params.is_wrapped && params.has_max_width_constraint {
            style.size.width = Some(auto_width.into());
        } else {
            let clamped = params
                .max_width_px
                .map_or(auto_width, |max_w| auto_width.min(max_w));
            style.size.width = Some(clamped.into());
        }
    }
}

struct WidthParams {
    user_wants_auto_width: bool,
    has_max_width_constraint: bool,
    container_width: Option<Pixels>,
    max_width_px: Option<Pixels>,
    is_wrapped: bool,
}

struct RenderParams {
    font: Font,
    font_size: Pixels,
    line_height: Pixels,
    scale_factor: f32,
    text_color: Hsla,
    highlight_text_color: Hsla,
}

fn register_actions(
    element: gpui::Stateful<gpui::Div>,
    window: &mut Window,
    state: &Entity<SelectableTextState>,
) -> gpui::Stateful<gpui::Div> {
    element
        .on_action(window.listener_for(state, SelectableTextState::left))
        .on_action(window.listener_for(state, SelectableTextState::right))
        .on_action(window.listener_for(state, SelectableTextState::up))
        .on_action(window.listener_for(state, SelectableTextState::down))
        .on_action(window.listener_for(state, SelectableTextState::home))
        .on_action(window.listener_for(state, SelectableTextState::end))
        .on_action(window.listener_for(state, SelectableTextState::select_left))
        .on_action(window.listener_for(state, SelectableTextState::select_right))
        .on_action(window.listener_for(state, SelectableTextState::select_up))
        .on_action(window.listener_for(state, SelectableTextState::select_down))
        .on_action(window.listener_for(state, SelectableTextState::select_all))
        .on_action(window.listener_for(state, SelectableTextState::move_to_start_of_line))
        .on_action(window.listener_for(state, SelectableTextState::move_to_end_of_line))
        .on_action(window.listener_for(state, SelectableTextState::select_to_start_of_line))
        .on_action(window.listener_for(state, SelectableTextState::select_to_end_of_line))
        .on_action(window.listener_for(state, SelectableTextState::move_to_start))
        .on_action(window.listener_for(state, SelectableTextState::move_to_end))
        .on_action(window.listener_for(state, SelectableTextState::select_to_start))
        .on_action(window.listener_for(state, SelectableTextState::select_to_end))
        .on_action(window.listener_for(state, SelectableTextState::move_to_previous_word))
        .on_action(window.listener_for(state, SelectableTextState::move_to_next_word))
        .on_action(window.listener_for(state, SelectableTextState::select_to_previous_word_start))
        .on_action(window.listener_for(state, SelectableTextState::select_to_next_word_end))
        .on_action(window.listener_for(state, SelectableTextState::copy))
}

fn register_mouse_handlers(
    element: gpui::Stateful<gpui::Div>,
    window: &mut Window,
    state: &Entity<SelectableTextState>,
) -> gpui::Stateful<gpui::Div> {
    element
        .on_mouse_down(
            MouseButton::Left,
            window.listener_for(state, SelectableTextState::on_mouse_down),
        )
        .on_mouse_up(
            MouseButton::Left,
            window.listener_for(state, SelectableTextState::on_mouse_up),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            window.listener_for(state, SelectableTextState::on_mouse_up),
        )
        .on_mouse_move(window.listener_for(state, SelectableTextState::on_mouse_move))
        .on_scroll_wheel(window.listener_for(state, SelectableTextState::on_scroll_wheel))
}

fn compute_line_offsets(text: &str) -> Vec<(usize, usize)> {
    text.split('\n')
        .scan(0, |start, line| {
            let end = *start + line.len();
            let offsets = (*start, end);
            *start = end + 1;
            Some(offsets)
        })
        .collect()
}

impl RenderOnce for SelectableText {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let params = self.compute_render_params(window);

        self.state.update(cx, |state, _cx| {
            state.set_multiline_params(params.line_height, self.line_clamp);
            state.set_wrap_mode(self.word_wrap);
            state.update_focus_state(window);
        });

        self.measure_text_width(
            &params.font,
            params.font_size,
            params.text_color,
            window,
            cx,
        );

        let (container_width, cached_wrap_width, focus_handle) = {
            let state = self.state.read(cx);
            (
                state.measured_max_line_width,
                state.cached_wrap_width,
                state.focus_handle.clone(),
            )
        };

        let user_wants_auto_width =
            matches!(self.style.size.width, None | Some(gpui::Length::Auto));

        let max_width_px: Option<Pixels> = match self.style.max_size.width {
            Some(gpui::Length::Definite(gpui::DefiniteLength::Absolute(abs))) => {
                Some(abs.to_pixels(window.rem_size()))
            }
            _ => None,
        };

        let has_max_width_constraint = self.style.max_size.width.is_some();

        let is_constrained = matches!(
            (cached_wrap_width, container_width),
            (Some(cached), Some(measured)) if measured + WIDTH_WRAP_BASE_MARGIN > cached
        );

        self.state.update(cx, |state, _cx| {
            state.is_constrained = is_constrained;
        });

        let width_params = WidthParams {
            user_wants_auto_width,
            has_max_width_constraint,
            container_width,
            max_width_px,
            is_wrapped: self.word_wrap,
        };

        let base = div()
            .id(self.id.clone())
            .min_w_0()
            .map(|mut this| {
                this.style().refine(&self.style);
                self.apply_auto_width(this.style(), &width_params);
                this
            })
            .key_context("SelectableText")
            .track_focus(&focus_handle)
            .cursor(CursorStyle::IBeam);

        let base = register_actions(base, window, &self.state);
        let base = register_mouse_handlers(base, window, &self.state);

        base.when(!self.word_wrap, |this| {
            self.render_unwrapped_list(this, &params, user_wants_auto_width, max_width_px, cx)
        })
        .when(self.word_wrap, |this| {
            self.render_wrapped_list(
                this,
                &params,
                user_wants_auto_width,
                has_max_width_constraint,
                max_width_px,
                window,
                cx,
            )
        })
    }
}

impl SelectableText {
    fn compute_render_params(&self, window: &Window) -> RenderParams {
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
        let line_height = window.round(line_height);
        let scale_factor = window.scale_factor();
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
        let text_color = self
            .style
            .text
            .color
            .unwrap_or_else(|| rgb(0xE8E4FF).into());
        let highlight_text_color = self
            .selection_color
            .unwrap_or_else(|| rgb_a(0x488BFF, 0.3).into());

        RenderParams {
            font,
            font_size,
            line_height,
            scale_factor,
            text_color,
            highlight_text_color,
        }
    }

    fn render_unwrapped_list(
        &self,
        container: gpui::Stateful<gpui::Div>,
        params: &RenderParams,
        user_wants_auto_width: bool,
        max_width_px: Option<Pixels>,
        cx: &mut App,
    ) -> gpui::Stateful<gpui::Div> {
        let font = params.font.clone();
        let (line_count, scroll_handle, measured_width) = {
            let state = self.state.read(cx);
            (
                state.line_count().max(1),
                state.scroll_handle.clone(),
                if user_wants_auto_width {
                    state.measured_max_line_width
                } else {
                    None
                },
            )
        };
        let state_entity = self.state.clone();
        let line_clamp = self.line_clamp;
        let needs_scroll = line_count > line_clamp;
        let text_color = params.text_color;
        let highlight_text_color = params.highlight_text_color;
        let line_height = params.line_height;
        let font_size = params.font_size;
        let scale_factor = params.scale_factor;
        let selection_rounded = self.selection_rounded;
        let selection_rounded_smoothing = self.selection_rounded_smoothing;
        let debug_character_bounds = self.debug_character_bounds;

        let list = uniform_list(
            self.id.clone(),
            line_count,
            move |visible_range, _window, cx| {
                let state = state_entity.read(cx);
                let value = state.get_text();
                let selected_range = state.selected_range.clone();
                let line_offsets = compute_line_offsets(&value);

                visible_range
                    .map(|line_idx| {
                        let (line_start, line_end) =
                            line_offsets.get(line_idx).copied().unwrap_or((0, 0));

                        // Get adjacent line offsets for corner radius computation
                        let prev_line_offsets = if line_idx > 0 {
                            line_offsets.get(line_idx - 1).copied()
                        } else {
                            None
                        };
                        let next_line_offsets = line_offsets.get(line_idx + 1).copied();

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
                            measured_width,
                            selection_rounded,
                            selection_rounded_smoothing,
                            prev_line_offsets,
                            next_line_offsets,
                            debug_character_bounds,
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
            if let Some(width) = measured_width {
                let auto_width = width + WIDTH_WRAP_BASE_MARGIN;
                let clamped = max_width_px.map_or(auto_width, |max_w| auto_width.min(max_w));
                list.style().size.width = Some(clamped.into());
            }
            list
        })
        .h(multiline_height(
            line_height,
            line_clamp.min(line_count).max(1),
            scale_factor,
        ));

        container.child(UniformListElement {
            state: self.state.clone(),
            child: list.into_any_element(),
            debug_wrapping: self.debug_wrapping,
        })
    }

    fn render_wrapped_list(
        &self,
        container: gpui::Stateful<gpui::Div>,
        params: &RenderParams,
        user_wants_auto_width: bool,
        has_max_width_constraint: bool,
        max_width_px: Option<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> gpui::Stateful<gpui::Div> {
        let font = params.font.clone();
        let (scroll_handle, cached_wrap_width, measured_max_line_width) = {
            let state = self.state.read(cx);
            (
                state.scroll_handle.clone(),
                state.cached_wrap_width,
                state.measured_max_line_width,
            )
        };
        let state_entity = self.state.clone();
        let line_clamp = self.line_clamp;
        let text_color = params.text_color;
        let highlight_text_color = params.highlight_text_color;
        let line_height = params.line_height;
        let font_size = params.font_size;
        let scale_factor = params.scale_factor;
        let selection_rounded = self.selection_rounded;
        let selection_rounded_smoothing = self.selection_rounded_smoothing;
        let debug_character_bounds = self.debug_character_bounds;

        let wrap_width = compute_wrap_width(
            cached_wrap_width,
            measured_max_line_width,
            max_width_px,
            user_wants_auto_width,
        );

        let (effective_width, use_relative_width) = compute_effective_width(
            user_wants_auto_width,
            has_max_width_constraint,
            cached_wrap_width,
            measured_max_line_width,
            max_width_px,
        );

        let visual_line_count = self.state.update(cx, |state, _cx| {
            state.using_auto_width = !use_relative_width && effective_width.is_some();
            // Cache render params for use in paint phase
            state.last_font = Some(font.clone());
            state.last_font_size = Some(font_size);
            state.last_text_color = Some(text_color);

            if state.needs_wrap_recompute || state.precomputed_visual_lines.is_empty() {
                state.needs_wrap_recompute = false;
                state.precompute_wrapped_lines(
                    wrap_width,
                    font_size,
                    font.clone(),
                    text_color,
                    window,
                )
            } else {
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
                let visual_lines = &state.precomputed_visual_lines;

                visible_range
                    .map(|visual_idx| {
                        // Get adjacent visual line offsets for corner radius computation
                        let prev_visual_line_offsets = if visual_idx > 0 {
                            visual_lines
                                .get(visual_idx - 1)
                                .map(|info| (info.start_offset, info.end_offset))
                        } else {
                            None
                        };
                        let next_visual_line_offsets = visual_lines
                            .get(visual_idx + 1)
                            .map(|info| (info.start_offset, info.end_offset));

                        WrappedLineElement {
                            state: state_entity.clone(),
                            visual_line_index: visual_idx,
                            text_color,
                            highlight_text_color,
                            line_height,
                            font_size,
                            font: font.clone(),
                            selected_range: selected_range.clone(),
                            selection_rounded,
                            selection_rounded_smoothing,
                            prev_visual_line_offsets,
                            next_visual_line_offsets,
                            debug_character_bounds,
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
            list.style().size.width = Some(gpui::relative(1.).into());
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

        container.child(UniformListElement {
            state: self.state.clone(),
            child: list.into_any_element(),
            debug_wrapping: self.debug_wrapping,
        })
    }
}

/// Registers default key bindings for selectable text.
pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("up", Up, None),
        KeyBinding::new("down", Down, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
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
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-a", SelectAll, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-c", Copy, None),
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
    use crate::utils::WIDTH_WRAP_BASE_MARGIN;
    use gpui::px;

    #[test]
    fn test_wrap_width_auto_uses_cached_directly() {
        let result = compute_wrap_width(Some(px(200.)), Some(px(400.)), None, true);
        assert_eq!(result, px(200.));
    }

    #[test]
    fn test_wrap_width_auto_uses_max_when_no_cached() {
        let result = compute_wrap_width(None, Some(px(500.)), Some(px(300.)), true);
        assert_eq!(result, px(300.) + WIDTH_WRAP_BASE_MARGIN);
    }

    #[test]
    fn test_wrap_width_fixed_uses_cached_directly() {
        let result = compute_wrap_width(Some(px(200.)), Some(px(400.)), None, false);
        assert_eq!(result, px(200.));
    }

    #[test]
    fn test_wrap_width_fixed_falls_back_to_max() {
        let result = compute_wrap_width(None, Some(px(400.)), Some(px(300.)), false);
        assert_eq!(result, px(300.) + WIDTH_WRAP_BASE_MARGIN);
    }

    #[test]
    fn test_wrap_width_defaults_to_max_when_nothing_available() {
        let result = compute_wrap_width(None, None, None, false);
        assert_eq!(result, Pixels::MAX + WIDTH_WRAP_BASE_MARGIN);
    }

    #[test]
    fn test_effective_width_uses_relative_on_first_render_with_max_constraint() {
        let (width, use_relative) = compute_effective_width(true, true, None, Some(px(400.)), None);
        assert_eq!(width, None);
        assert!(use_relative);
    }

    #[test]
    fn test_effective_width_uses_auto_when_text_fits() {
        let (width, use_relative) =
            compute_effective_width(true, true, Some(px(500.)), Some(px(400.)), None);
        assert_eq!(width, Some(px(400.) + WIDTH_WRAP_BASE_MARGIN));
        assert!(!use_relative);
    }

    #[test]
    fn test_effective_width_uses_relative_when_text_exceeds_available() {
        let (width, use_relative) =
            compute_effective_width(true, true, Some(px(300.)), Some(px(400.)), None);
        assert_eq!(width, None);
        assert!(use_relative);
    }

    #[test]
    fn test_effective_width_uses_cached_when_no_measured() {
        let (width, use_relative) = compute_effective_width(true, true, Some(px(300.)), None, None);
        assert_eq!(width, Some(px(300.)));
        assert!(!use_relative);
    }

    #[test]
    fn test_effective_width_uses_relative_when_nothing_available() {
        let (width, use_relative) = compute_effective_width(true, true, None, None, None);
        assert_eq!(width, None);
        assert!(use_relative);
    }

    #[test]
    fn test_effective_width_uses_measured_when_no_max_constraint() {
        let (width, use_relative) =
            compute_effective_width(true, false, None, Some(px(400.)), None);
        assert_eq!(width, Some(px(400.) + WIDTH_WRAP_BASE_MARGIN));
        assert!(!use_relative);
    }

    #[test]
    fn test_effective_width_clamped_by_absolute_max_when_no_relative_constraint() {
        let (width, use_relative) =
            compute_effective_width(true, false, None, Some(px(400.)), Some(px(300.)));
        assert_eq!(width, Some(px(300.)));
        assert!(!use_relative);
    }

    #[test]
    fn test_effective_width_none_when_not_auto_width() {
        let (width, use_relative) =
            compute_effective_width(false, false, None, Some(px(400.)), None);
        assert_eq!(width, None);
        assert!(!use_relative);
    }

    #[test]
    fn test_effective_width_none_when_not_measured_yet_no_max_constraint() {
        let (width, use_relative) = compute_effective_width(true, false, None, None, None);
        assert_eq!(width, None);
        assert!(!use_relative);
    }

    #[test]
    fn test_wrap_width_fallback_uses_base_margin() {
        // Fallback estimate uses base margin
        let result = compute_wrap_width(None, Some(px(200.)), Some(px(300.)), true);
        assert_eq!(result, px(300.) + WIDTH_WRAP_BASE_MARGIN);
    }

    #[test]
    fn test_effective_width_always_uses_base_margin() {
        // compute_effective_width always uses just base margin (no whitespace)
        let (width, use_relative) =
            compute_effective_width(true, false, None, Some(px(400.)), None);
        assert_eq!(width, Some(px(400.) + WIDTH_WRAP_BASE_MARGIN));
        assert!(!use_relative);
    }
}
