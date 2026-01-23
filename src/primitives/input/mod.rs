use std::ops::Range;
use std::sync::Arc;

use gpui::{
    App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, ElementInputHandler, Entity,
    FocusHandle, Focusable, GlobalElementId, Hsla, InspectorElementId, InteractiveElement,
    IntoElement, KeyBinding, LayoutId, MouseButton, MouseMoveEvent, PaintQuad, ParentElement,
    Pixels, Refineable, RenderOnce, ShapedLine, SharedString, Style, StyleRefinement, Styled,
    TextRun, UnderlineStyle, Window, WrappedLine, div, fill, hsla, point, prelude::FluentBuilder,
    px, relative, rgb, size,
};

mod cursor_blink;
mod state;
pub mod text_transforms;

pub use cursor_blink::CursorBlink;
pub use state::*;

use crate::utils::rgb_a;

type TransformTextFn = Arc<dyn Fn(char) -> char + Send + Sync>;

#[derive(IntoElement)]
pub struct Input {
    id: ElementId,
    state: Entity<InputState>,
    disabled: bool,
    max_lines: usize,
    wrap: bool,
    /// When true, use shift+enter for newlines instead of enter
    newline_on_shift_enter: bool,
    pub(crate) placeholder: SharedString,
    placeholder_text_color: Option<Hsla>,
    selection_color: Option<Hsla>,
    transform_text: Option<TransformTextFn>,
    style: StyleRefinement,
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Input {
    pub fn new(id: impl Into<ElementId>, state: Entity<InputState>) -> Self {
        Self {
            id: id.into(),
            state,
            disabled: false,
            max_lines: 1,
            wrap: false,
            newline_on_shift_enter: false,
            placeholder: "Type here...".into(),
            placeholder_text_color: None,
            selection_color: None,
            transform_text: None,
            style: StyleRefinement::default(),
        }
    }

    /// Sets the maximum number of visible lines before scrolling.
    /// - `max_lines == 1` (default): single-line input
    /// - `max_lines > 1`: multi-line input using uniform_list for efficient rendering
    /// - Use `usize::MAX` for practically unlimited lines
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = max_lines.max(1);
        self
    }

    /// Enables multi-line mode with unconstrained height (no scrolling).
    /// Equivalent to `.max_lines(usize::MAX)`.
    pub fn multiline(mut self) -> Self {
        self.max_lines = usize::MAX;
        self
    }

    /// Enables word wrapping for multi-line input.
    /// Text will wrap at word boundaries when it exceeds the input width.
    /// Only effective when `max_lines > 1`.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// When enabled, use shift+enter to insert newlines instead of enter.
    /// This is useful for form inputs where enter should submit the form.
    /// Only effective when `max_lines > 1`.
    pub fn newline_on_shift_enter(mut self, enabled: bool) -> Self {
        self.newline_on_shift_enter = enabled;
        self
    }

    pub fn transform_text(
        mut self,
        transform: impl Fn(char) -> char + Send + Sync + 'static,
    ) -> Self {
        self.transform_text = Some(Arc::new(transform));
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn read_text(&self, cx: &mut App) -> SharedString {
        self.state.read(cx).value()
    }
}

struct TextElement {
    input: Entity<InputState>,
    placeholder: SharedString,
    text_color: Hsla,
    placeholder_text_color: Hsla,
    highlight_text_color: Hsla,
    line_height: Pixels,
    transform_text: Option<TransformTextFn>,
    cursor_visible: bool,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = self.line_height.into();

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.value();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            (self.placeholder.clone(), self.placeholder_text_color)
        } else if let Some(transform) = &self.transform_text {
            let transformed: String = content.chars().map(|c| transform(c)).collect();
            (transformed.into(), self.text_color)
        } else {
            (content, self.text_color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            let height = bounds.bottom() - bounds.top();
            let adjusted_height = height * 0.8;
            let height_diff = height - adjusted_height;

            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top() + height_diff / 2.),
                        size(px(1.), adjusted_height),
                    ),
                    self.text_color,
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    self.highlight_text_color,
                )),
                None,
            )
        };

        PrepaintState {
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();

        // Register window-level mouse move listener to handle selection
        // even when cursor leaves the input bounds during drag
        let input = self.input.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
            if phase == DispatchPhase::Capture {
                return;
            }

            input.update(cx, |input, cx| {
                if input.is_selecting {
                    input.select_to(input.index_for_mouse_position(event.position), cx);
                }
            });
        });

        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }

        let line = prepaint.line.take().unwrap();
        line.paint(
            bounds.origin,
            self.line_height,
            gpui::TextAlign::Left,
            None,
            window,
            cx,
        )
        .unwrap();

        if focus_handle.is_focused(window)
            && self.cursor_visible
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

/// Wrapper element for multi-line input that handles input registration
/// and contains the uniform_list for rendering lines.
struct MultiLineInputElement {
    id: ElementId,
    input: Entity<InputState>,
    line_count: usize,
    line_offsets: Vec<(usize, usize)>,
    text_color: Hsla,
    placeholder_text_color: Hsla,
    highlight_text_color: Hsla,
    line_height: Pixels,
    transform_text: Option<TransformTextFn>,
    cursor_visible: bool,
    selected_range: Range<usize>,
    cursor_offset: usize,
    placeholder: SharedString,
    is_empty: bool,
    max_lines: usize,
    wrap: bool,
    value: SharedString,
}

struct MultiLinePrepaintState {
    /// For non-wrapped mode: individual line elements
    line_elements: Vec<LineElement>,
    /// For wrapped mode: the wrapped lines from shape_text
    wrapped_lines: Option<WrappedTextState>,
}

struct WrappedTextState {
    lines: Vec<WrappedLine>,
    /// Maps visual line index to (logical_line_start_offset, visual_line_start_offset_in_logical)
    visual_line_info: Vec<VisualLineInfo>,
}

#[derive(Clone)]
struct VisualLineInfo {
    /// Byte offset where this visual line starts in the full text
    start_offset: usize,
    /// Byte offset where this visual line ends in the full text
    end_offset: usize,
    /// The wrapped line this belongs to
    wrapped_line_index: usize,
    /// The visual line index within the wrapped line (for multi-wrap scenarios)
    visual_index_in_wrapped: usize,
}

impl IntoElement for MultiLineInputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for MultiLineInputElement {
    type RequestLayoutState = ();
    type PrepaintState = MultiLinePrepaintState;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        // Height based on visible lines
        let visible_lines = self.max_lines.min(self.line_count).max(1);
        style.size.height = (self.line_height * visible_lines as f32).into();

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        if self.wrap {
            // Wrapped mode: use shape_text to get wrapped lines
            let style = window.text_style();
            let font_size = style.font_size.to_pixels(window.rem_size());

            let display_text: SharedString = if self.is_empty {
                self.placeholder.clone()
            } else if let Some(transform) = &self.transform_text {
                let transformed: String = self.value.chars().map(|c| transform(c)).collect();
                transformed.into()
            } else {
                self.value.clone()
            };

            let text_color = if self.is_empty {
                self.placeholder_text_color
            } else {
                self.text_color
            };

            let run = TextRun {
                len: display_text.len(),
                font: style.font(),
                color: text_color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };

            // Shape text with wrapping
            let wrapped_lines = window
                .text_system()
                .shape_text(
                    display_text,
                    font_size,
                    &[run],
                    Some(bounds.size.width),
                    None,
                )
                .unwrap_or_default();

            // Build visual line info - each WrappedLine can contain multiple visual lines
            // due to wrap boundaries
            let mut visual_line_info = Vec::new();
            let mut text_offset = 0;

            for (wrapped_idx, wrapped_line) in wrapped_lines.iter().enumerate() {
                let line_len = wrapped_line.len();
                let wrap_boundaries = &wrapped_line.wrap_boundaries;

                if wrap_boundaries.is_empty() {
                    // No wrapping within this line
                    visual_line_info.push(VisualLineInfo {
                        start_offset: text_offset,
                        end_offset: text_offset + line_len,
                        wrapped_line_index: wrapped_idx,
                        visual_index_in_wrapped: 0,
                    });
                } else {
                    // Line has wrap boundaries - create visual line for each segment
                    let mut segment_start = 0;
                    for (visual_idx, boundary) in wrap_boundaries.iter().enumerate() {
                        let segment_end = boundary.glyph_ix;
                        visual_line_info.push(VisualLineInfo {
                            start_offset: text_offset + segment_start,
                            end_offset: text_offset + segment_end,
                            wrapped_line_index: wrapped_idx,
                            visual_index_in_wrapped: visual_idx,
                        });
                        segment_start = segment_end;
                    }
                    // Add final segment after last wrap boundary
                    visual_line_info.push(VisualLineInfo {
                        start_offset: text_offset + segment_start,
                        end_offset: text_offset + line_len,
                        wrapped_line_index: wrapped_idx,
                        visual_index_in_wrapped: wrap_boundaries.len(),
                    });
                }

                // Account for newline character between logical lines
                text_offset += line_len + 1;
            }

            MultiLinePrepaintState {
                line_elements: Vec::new(),
                wrapped_lines: Some(WrappedTextState {
                    lines: wrapped_lines.into_vec(),
                    visual_line_info,
                }),
            }
        } else {
            // Non-wrapped mode: create line elements for all visible lines
            let line_elements: Vec<LineElement> = (0..self.line_count)
                .map(|line_idx| {
                    let (line_start, line_end) =
                        self.line_offsets.get(line_idx).copied().unwrap_or((0, 0));

                    LineElement {
                        input: self.input.clone(),
                        line_index: line_idx,
                        line_start_offset: line_start,
                        line_end_offset: line_end,
                        text_color: self.text_color,
                        placeholder_text_color: self.placeholder_text_color,
                        highlight_text_color: self.highlight_text_color,
                        line_height: self.line_height,
                        transform_text: self.transform_text.clone(),
                        cursor_visible: self.cursor_visible,
                        selected_range: self.selected_range.clone(),
                        cursor_offset: self.cursor_offset,
                        placeholder: self.placeholder.clone(),
                        is_empty: self.is_empty,
                    }
                })
                .collect();

            MultiLinePrepaintState {
                line_elements,
                wrapped_lines: None,
            }
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();

        // Register window-level mouse move listener to handle selection
        // even when cursor leaves the input bounds during drag
        let input = self.input.clone();
        let line_height = self.line_height;
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
            if phase == DispatchPhase::Capture {
                return;
            }

            input.update(cx, |input, cx| {
                if input.is_selecting {
                    input.select_to_multiline(event.position, line_height, cx);
                }
            });
        });

        // Register input handler for the entire multi-line input area
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        if let Some(wrapped_state) = &prepaint.wrapped_lines {
            // Paint wrapped lines
            self.paint_wrapped_lines(bounds, wrapped_state, window, cx);

            // Store bounds for mouse position calculation
            self.input.update(cx, |input, _cx| {
                input.last_bounds = Some(bounds);
                // Clear multiline layouts for wrapped mode (not yet supported)
                input.multiline_layouts.clear();
                input.multiline_line_bounds.clear();
            });
        } else {
            // Paint non-wrapped lines and collect ShapedLine layouts
            let mut layouts = Vec::with_capacity(prepaint.line_elements.len());
            let mut line_bounds_vec = Vec::with_capacity(prepaint.line_elements.len());

            let mut y_offset = bounds.origin.y;
            for line_element in &mut prepaint.line_elements {
                let line_bounds = Bounds::new(
                    point(bounds.origin.x, y_offset),
                    size(bounds.size.width, self.line_height),
                );

                // Prepaint and paint the line element
                let mut layout_state = ();
                let mut line_prepaint =
                    line_element.prepaint(None, None, line_bounds, &mut layout_state, window, cx);

                // Clone the ShapedLine before paint consumes it
                if let Some(ref shaped_line) = line_prepaint.line {
                    layouts.push(shaped_line.clone());
                    line_bounds_vec.push(line_bounds);
                }

                line_element.paint(
                    None,
                    None,
                    line_bounds,
                    &mut layout_state,
                    &mut line_prepaint,
                    window,
                    cx,
                );

                y_offset += self.line_height;
            }

            // Store bounds and layouts for mouse position calculation
            self.input.update(cx, |input, _cx| {
                input.last_bounds = Some(bounds);
                input.multiline_layouts = layouts;
                input.multiline_line_bounds = line_bounds_vec;
            });
        }
    }
}

impl MultiLineInputElement {
    fn paint_wrapped_lines(
        &self,
        bounds: Bounds<Pixels>,
        wrapped_state: &WrappedTextState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        let is_focused = focus_handle.is_focused(window);

        // Paint each wrapped line
        let mut y_offset = bounds.origin.y;
        for wrapped_line in &wrapped_state.lines {
            // Calculate height for this wrapped line (accounts for internal wrapping)
            let wrap_count = wrapped_line.wrap_boundaries.len() + 1;
            let line_total_height = self.line_height * wrap_count as f32;

            // Paint selection background if needed
            // TODO: implement selection painting for wrapped lines

            // Paint the wrapped line
            wrapped_line
                .paint(
                    point(bounds.origin.x, y_offset),
                    self.line_height,
                    gpui::TextAlign::Left,
                    Some(bounds),
                    window,
                    cx,
                )
                .ok();

            y_offset += line_total_height;
        }

        // Paint cursor if focused and visible
        if is_focused && self.cursor_visible && !self.is_empty {
            self.paint_cursor_for_wrapped(bounds, wrapped_state, window);
        }
    }

    fn paint_cursor_for_wrapped(
        &self,
        bounds: Bounds<Pixels>,
        wrapped_state: &WrappedTextState,
        window: &mut Window,
    ) {
        // Find which visual line the cursor is on
        let cursor = self.cursor_offset;
        let mut visual_line_idx = 0;
        for (idx, info) in wrapped_state.visual_line_info.iter().enumerate() {
            if cursor >= info.start_offset && cursor <= info.end_offset {
                visual_line_idx = idx;
                break;
            }
        }

        if let Some(info) = wrapped_state.visual_line_info.get(visual_line_idx) {
            if let Some(wrapped_line) = wrapped_state.lines.get(info.wrapped_line_index) {
                // Calculate cursor position within the line
                let local_cursor = cursor.saturating_sub(info.start_offset);

                // Get x position for cursor
                // For wrapped lines, we need to account for wrap boundaries
                let cursor_x = if info.visual_index_in_wrapped == 0 {
                    wrapped_line.unwrapped_layout.x_for_index(local_cursor)
                } else {
                    // For subsequent visual lines within a wrap, adjust for wrap boundary
                    let wrap_start = if info.visual_index_in_wrapped > 0 {
                        wrapped_line
                            .wrap_boundaries
                            .get(info.visual_index_in_wrapped - 1)
                            .map(|b| b.glyph_ix)
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    let offset_in_visual_line = cursor.saturating_sub(info.start_offset);
                    wrapped_line
                        .unwrapped_layout
                        .x_for_index(wrap_start + offset_in_visual_line)
                        - wrapped_line.unwrapped_layout.x_for_index(wrap_start)
                };

                // Calculate y position
                let mut y_offset = bounds.origin.y;
                for (idx, line) in wrapped_state.lines.iter().enumerate() {
                    if idx == info.wrapped_line_index {
                        y_offset += self.line_height * info.visual_index_in_wrapped as f32;
                        break;
                    }
                    let wrap_count = line.wrap_boundaries.len() + 1;
                    y_offset += self.line_height * wrap_count as f32;
                }

                let height = self.line_height;
                let adjusted_height = height * 0.8;
                let height_diff = height - adjusted_height;

                window.paint_quad(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_x, y_offset + height_diff / 2.),
                        size(px(1.), adjusted_height),
                    ),
                    self.text_color,
                ));
            }
        }
    }
}

/// Element for rendering a single line in a multi-line input.
/// Used by uniform_list to render only the visible lines.
struct LineElement {
    input: Entity<InputState>,
    line_index: usize,
    line_start_offset: usize,
    line_end_offset: usize,
    text_color: Hsla,
    placeholder_text_color: Hsla,
    highlight_text_color: Hsla,
    line_height: Pixels,
    transform_text: Option<TransformTextFn>,
    cursor_visible: bool,
    /// The global selection range from InputState
    selected_range: Range<usize>,
    /// The cursor position (byte offset in full text)
    cursor_offset: usize,
    /// Placeholder text (only shown on first line when empty)
    placeholder: SharedString,
    /// Whether the input is empty
    is_empty: bool,
}

struct LinePrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for LineElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for LineElement {
    type RequestLayoutState = ();
    type PrepaintState = LinePrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = self.line_height.into();

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let full_value = input.value();
        let style = window.text_style();

        // Get line content as owned String to avoid lifetime issues
        let line_content: String =
            full_value[self.line_start_offset..self.line_end_offset].to_string();

        // Determine display text and color
        let (display_text, text_color): (SharedString, Hsla) =
            if self.is_empty && self.line_index == 0 {
                // Show placeholder on first line when empty
                (self.placeholder.clone(), self.placeholder_text_color)
            } else if let Some(transform) = &self.transform_text {
                let transformed: String = line_content.chars().map(|c| transform(c)).collect();
                (transformed.into(), self.text_color)
            } else {
                (line_content.clone().into(), self.text_color)
            };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &[run], None);

        // Calculate selection and cursor for this line

        // Check if cursor is on this line
        let cursor_on_this_line = self.cursor_offset >= self.line_start_offset
            && self.cursor_offset <= self.line_end_offset;

        // Calculate local cursor position within this line
        let local_cursor = if cursor_on_this_line {
            Some(self.cursor_offset - self.line_start_offset)
        } else {
            None
        };

        // Calculate selection intersection with this line
        let selection_intersects = self.selected_range.start < self.line_end_offset
            && self.selected_range.end > self.line_start_offset;

        let (selection, cursor) = if !self.selected_range.is_empty() && selection_intersects {
            // Calculate local selection range
            let local_start = self
                .selected_range
                .start
                .saturating_sub(self.line_start_offset)
                .min(line_content.len());
            let local_end = self
                .selected_range
                .end
                .saturating_sub(self.line_start_offset)
                .min(line_content.len());

            (
                Some(fill(
                    Bounds::from_corners(
                        point(bounds.left() + line.x_for_index(local_start), bounds.top()),
                        point(bounds.left() + line.x_for_index(local_end), bounds.bottom()),
                    ),
                    self.highlight_text_color,
                )),
                None,
            )
        } else if let Some(local_cursor) = local_cursor {
            let cursor_pos = line.x_for_index(local_cursor);
            let height = bounds.bottom() - bounds.top();
            let adjusted_height = height * 0.8;
            let height_diff = height - adjusted_height;

            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top() + height_diff / 2.),
                        size(px(1.), adjusted_height),
                    ),
                    self.text_color,
                )),
            )
        } else {
            (None, None)
        };

        LinePrepaintState {
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();

        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }

        let line = prepaint.line.take().unwrap();
        line.paint(
            bounds.origin,
            self.line_height,
            gpui::TextAlign::Left,
            None,
            window,
            cx,
        )
        .unwrap();

        if focus_handle.is_focused(window)
            && self.cursor_visible
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let is_multiline = self.max_lines > 1;

        // Calculate line_height early so we can pass it to state
        let text_style = &self.style.text;
        let line_height = text_style
            .line_height
            .map(|this| {
                this.to_pixels(
                    text_style
                        .font_size
                        .unwrap_or_else(|| window.text_style().font_size),
                    window.rem_size(),
                )
            })
            .unwrap_or_else(|| window.line_height());

        // Update focus state and multiline params
        self.state.update(cx, |state, cx| {
            state.update_focus_state(window, cx);
            state.set_multiline_params(is_multiline, line_height);
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
            .on_action(window.listener_for(&self.state, InputState::backspace))
            .on_action(window.listener_for(&self.state, InputState::delete))
            .on_action(window.listener_for(&self.state, InputState::left))
            .on_action(window.listener_for(&self.state, InputState::right))
            .on_action(window.listener_for(&self.state, InputState::select_left))
            .on_action(window.listener_for(&self.state, InputState::select_right))
            .on_action(window.listener_for(&self.state, InputState::select_all))
            .on_action(window.listener_for(&self.state, InputState::home))
            .on_action(window.listener_for(&self.state, InputState::end))
            .on_action(window.listener_for(&self.state, InputState::show_character_palette))
            .on_action(window.listener_for(&self.state, InputState::paste))
            .on_action(window.listener_for(&self.state, InputState::cut))
            .on_action(window.listener_for(&self.state, InputState::copy))
            // Multi-line navigation actions
            .when(is_multiline, |this| {
                this.on_action(window.listener_for(&self.state, InputState::up))
                    .on_action(window.listener_for(&self.state, InputState::down))
                    .on_action(window.listener_for(&self.state, InputState::select_up))
                    .on_action(window.listener_for(&self.state, InputState::select_down))
                    // Bind newline action based on configuration
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
            .when(is_multiline, |this| {
                // Multi-line mode: use MultiLineInputElement for proper input handling
                let line_count = state.line_count().max(1);
                let selected_range = state.selected_range.clone();
                let cursor_offset = state.cursor_offset();
                let is_empty = state.value().is_empty();

                // Pre-compute line offsets
                let mut line_offsets: Vec<(usize, usize)> = Vec::with_capacity(line_count);
                let value = state.value();
                let mut start = 0;
                for line in value.split('\n') {
                    let end = start + line.len();
                    line_offsets.push((start, end));
                    start = end + 1; // +1 for the newline character
                }

                this.child(MultiLineInputElement {
                    id: self.id.clone(),
                    input: self.state.clone(),
                    line_count,
                    line_offsets,
                    text_color,
                    placeholder_text_color,
                    highlight_text_color,
                    line_height,
                    transform_text: self.transform_text.clone(),
                    cursor_visible,
                    selected_range,
                    cursor_offset,
                    placeholder: self.placeholder.clone(),
                    is_empty,
                    max_lines: self.max_lines,
                    wrap: self.wrap,
                    value: state.value(),
                })
            })
            .when(!is_multiline, |this| {
                // Single-line mode: use TextElement directly
                this.child(TextElement {
                    input: self.state.clone(),
                    placeholder: self.placeholder,
                    text_color,
                    placeholder_text_color,
                    highlight_text_color,
                    line_height,
                    transform_text: self.transform_text,
                    cursor_visible,
                })
            })
    }
}

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("up", Up, None),
        KeyBinding::new("down", Down, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("shift-up", SelectUp, None),
        KeyBinding::new("shift-down", SelectDown, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, None),
        KeyBinding::new("enter", InsertNewline, None),
        KeyBinding::new("shift-enter", InsertNewlineShift, None),
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
