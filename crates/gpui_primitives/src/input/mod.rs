use std::ops::Range;
use std::sync::Arc;

use gpui::{
    AnyElement, App, Bounds, CursorStyle, DispatchPhase, Element, ElementId, ElementInputHandler,
    Entity, FocusHandle, Focusable, GlobalElementId, Hsla, InspectorElementId, InteractiveElement,
    IntoElement, KeyBinding, LayoutId, MouseButton, MouseMoveEvent, Overflow, PaintQuad,
    ParentElement, Pixels, Refineable, RenderOnce, ShapedLine, SharedString, Style,
    StyleRefinement, Styled, TextRun, UnderlineStyle, Window, div, fill, hsla, point,
    prelude::FluentBuilder, px, relative, rgb, size, uniform_list,
};

mod cursor_blink;
mod state;
pub mod text_transforms;

pub use cursor_blink::CursorBlink;
pub use state::{
    Backspace, Copy, Cut, Delete, Down, End, Home, InputState, InsertNewline, InsertNewlineShift,
    Left, MapTextFn, Paste, Quit, Right, SelectAll, SelectDown, SelectLeft, SelectRight, SelectUp,
    ShowCharacterPalette, Up, VisibleLineInfo, VisualLineInfo,
};

use crate::utils::rgb_a;

type TransformTextFn = Arc<dyn Fn(char) -> char + Send + Sync>;

/// Calculates the height for a multiline input, floored to the nearest 0.5 pixel.
/// This prevents slight layout shifts caused by subpixel height variations.
fn multiline_height(line_height: Pixels, line_count: usize) -> Pixels {
    px((line_height.to_f64() as f32 * line_count as f32 * 2.0).floor() / 2.0)
}

#[derive(IntoElement)]
pub struct Input {
    id: ElementId,
    state: Entity<InputState>,
    disabled: bool,
    max_lines: usize,
    wrap: bool,
    /// When true, use shift+enter for newlines instead of enter
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
            map_text: None,
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

    /// Transform the text value whenever it changes.
    /// Unlike `transform_text`, this actually modifies the stored value.
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

    pub fn get_placeholder(&self) -> &SharedString {
        &self.placeholder
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

        // Store visible line info for mouse hit testing
        self.input.update(cx, |input, _cx| {
            input.visible_lines_info.push(VisibleLineInfo {
                line_index: self.line_index,
                bounds,
                shaped_line: line.clone(),
            });
        });

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

/// Element for rendering a single visual line in wrapped multi-line input.
/// Used by uniform_list to render only visible wrapped lines.
struct WrappedLineElement {
    input: Entity<InputState>,
    /// Index into precomputed_visual_lines
    visual_line_index: usize,
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
    /// Placeholder text (only shown on first visual line when empty)
    placeholder: SharedString,
    /// Whether the input is empty
    is_empty: bool,
}

struct WrappedLinePrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for WrappedLineElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for WrappedLineElement {
    type RequestLayoutState = ();
    type PrepaintState = WrappedLinePrepaintState;

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
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());

        // Get the visual line info
        let visual_info = input
            .precomputed_visual_lines
            .get(self.visual_line_index)
            .cloned();

        let Some(info) = visual_info else {
            return WrappedLinePrepaintState {
                line: None,
                cursor: None,
                selection: None,
            };
        };

        // Get display text for this visual line
        let (display_text, text_color): (SharedString, Hsla) =
            if self.is_empty && self.visual_line_index == 0 {
                // Show placeholder on first line when empty
                (self.placeholder.clone(), self.placeholder_text_color)
            } else {
                let value = input.value();
                let segment = &value[info.start_offset..info.end_offset];
                let text: SharedString = if let Some(transform) = &self.transform_text {
                    segment
                        .chars()
                        .map(|c| transform(c))
                        .collect::<String>()
                        .into()
                } else {
                    segment.to_string().into()
                };
                (text, self.text_color)
            };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let line = window
            .text_system()
            .shape_line(display_text, font_size, &[run], None);

        // Calculate selection and cursor for this visual line
        let line_start = info.start_offset;
        let line_end = info.end_offset;
        let line_len = line_end - line_start;

        // Check if cursor is on this visual line
        let cursor_on_this_line =
            self.cursor_offset >= line_start && self.cursor_offset <= line_end;

        let local_cursor = if cursor_on_this_line {
            Some(self.cursor_offset - line_start)
        } else {
            None
        };

        // Calculate selection intersection with this visual line
        let selection_intersects =
            self.selected_range.start < line_end && self.selected_range.end > line_start;

        let (selection, cursor) = if !self.selected_range.is_empty() && selection_intersects {
            let local_start = self
                .selected_range
                .start
                .saturating_sub(line_start)
                .min(line_len);
            let local_end = self
                .selected_range
                .end
                .saturating_sub(line_start)
                .min(line_len);

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

        WrappedLinePrepaintState {
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

        let Some(line) = prepaint.line.take() else {
            return;
        };

        // Store visible line info for mouse hit testing
        self.input.update(cx, |input, _cx| {
            input.visible_lines_info.push(VisibleLineInfo {
                line_index: self.visual_line_index,
                bounds,
                shaped_line: line.clone(),
            });
        });

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

/// Wrapper element that contains a uniform_list and registers the input handler.
/// This is needed because uniform_list doesn't provide a paint hook where we can
/// call window.handle_input().
struct UniformListInputElement {
    input: Entity<InputState>,
    child: AnyElement,
}

impl IntoElement for UniformListInputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for UniformListInputElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

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
        let layout_id = self.child.request_layout(window, cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();

        // Register input handler for the entire input area
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        // Register window-level mouse move listener to handle selection
        // even when cursor leaves the input bounds during drag
        let input = self.input.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
            if phase == DispatchPhase::Capture {
                return;
            }

            input.update(cx, |input, cx| {
                if input.is_selecting {
                    if let Some(line_height) = input.line_height {
                        input.select_to_multiline(event.position, line_height, cx);
                    }
                }
            });
        });

        // Clear visible_lines_info before painting - LineElement will repopulate it
        self.input.update(cx, |input, _cx| {
            input.visible_lines_info.clear();
        });

        // Paint the child (uniform_list)
        self.child.paint(window, cx);

        // Store bounds and cache width for mouse position calculations and wrapped line computation
        self.input.update(cx, |input, _cx| {
            input.last_bounds = Some(bounds);
            input.cached_wrap_width = Some(bounds.size.width);
        });
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

        // Update focus state, multiline params, and map_text
        let map_text = self.map_text.clone();
        self.state.update(cx, |state, cx| {
            state.update_focus_state(window, cx);
            state.set_multiline_params(is_multiline, line_height, self.max_lines);
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
            .when(is_multiline && !self.wrap, |this| {
                // Multi-line non-wrapped mode: use uniform_list for efficient scrolling
                let line_count = state.line_count().max(1);
                let scroll_handle = state.scroll_handle.clone();
                let input_state = self.state.clone();
                let transform_text = self.transform_text.clone();
                let placeholder = self.placeholder.clone();
                let max_lines = self.max_lines;

                // Only enable scroll tracking when content exceeds max_lines
                let needs_scroll = line_count > max_lines;

                let list = uniform_list(
                    self.id.clone(),
                    line_count,
                    move |visible_range, _window, cx| {
                        let state = input_state.read(cx);
                        let value = state.value();
                        let selected_range = state.selected_range.clone();
                        let cursor_offset = state.cursor_offset();
                        let is_empty = value.is_empty();

                        // Pre-compute line offsets
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
                        // Override the default overflow.y = Scroll to prevent scrolling
                        list.style().overflow.y = Some(Overflow::Hidden);
                    }
                    list
                })
                .h(multiline_height(
                    line_height,
                    max_lines.min(line_count).max(1),
                ));

                this.child(UniformListInputElement {
                    input: self.state.clone(),
                    child: list.into_any_element(),
                })
            })
            .when(is_multiline && self.wrap, |this| {
                // Multi-line wrapped mode: use uniform_list with visual lines
                // Note: We capture what we need before entering the closure to avoid borrow issues
                let scroll_handle = self.state.read(cx).scroll_handle.clone();
                let cached_wrap_width = self.state.read(cx).cached_wrap_width;
                let input_state = self.state.clone();
                let transform_text = self.transform_text.clone();
                let placeholder = self.placeholder.clone();
                let max_lines = self.max_lines;

                // Pre-compute visual lines using cached width (or default)
                let wrap_width = cached_wrap_width.unwrap_or(px(300.));
                let visual_line_count = self.state.update(cx, |state, _cx| {
                    let count = state.precompute_wrapped_lines(wrap_width, text_color, window);
                    state.is_wrapped = true;
                    count
                });

                // Only enable scroll tracking when content exceeds max_lines
                let needs_scroll = visual_line_count > max_lines;

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
                        // Override the default overflow.y = Scroll to prevent scrolling
                        list.style().overflow.y = Some(Overflow::Hidden);
                    }
                    list
                })
                .h(multiline_height(
                    line_height,
                    max_lines.min(visual_line_count).max(1),
                ));

                this.child(UniformListInputElement {
                    input: self.state.clone(),
                    child: list.into_any_element(),
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
