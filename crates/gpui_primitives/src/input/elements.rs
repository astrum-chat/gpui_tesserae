use std::ops::Range;

use gpui::{
    AnyElement, App, Bounds, DispatchPhase, Element, ElementId, ElementInputHandler, Entity, Font,
    GlobalElementId, Hsla, InspectorElementId, IntoElement, LayoutId, MouseMoveEvent, PaintQuad,
    Pixels, ShapedLine, SharedString, Style, TextRun, UnderlineStyle, Window, fill, point, px,
    relative, size,
};

use super::state::InputState;
use super::text_navigation::TextNavigation;
use super::{
    TransformTextFn, VisibleLineInfo, WRAP_WIDTH_EPSILON, should_show_trailing_whitespace,
};

/// Creates a cursor quad for rendering.
pub(crate) fn make_cursor_quad(
    bounds: Bounds<Pixels>,
    cursor_x: Pixels,
    scroll_offset: Pixels,
    text_color: Hsla,
) -> PaintQuad {
    let height = bounds.bottom() - bounds.top();
    let adjusted_height = height * 0.8;
    let height_diff = height - adjusted_height;
    fill(
        gpui::Bounds::new(
            point(
                bounds.left() + cursor_x - scroll_offset,
                bounds.top() + height_diff / 2.,
            ),
            size(px(1.), adjusted_height),
        ),
        text_color,
    )
}

/// Creates a selection quad for rendering.
pub(crate) fn make_selection_quad(
    bounds: Bounds<Pixels>,
    start_x: Pixels,
    end_x: Pixels,
    scroll_offset: Pixels,
    highlight_color: Hsla,
) -> PaintQuad {
    fill(
        Bounds::from_corners(
            point(bounds.left() + start_x - scroll_offset, bounds.top()),
            point(bounds.left() + end_x - scroll_offset, bounds.bottom()),
        ),
        highlight_color,
    )
}

/// Element for rendering a single-line input.
pub(crate) struct TextElement {
    pub input: Entity<InputState>,
    pub placeholder: SharedString,
    pub text_color: Hsla,
    pub placeholder_text_color: Hsla,
    pub highlight_text_color: Hsla,
    pub line_height: Pixels,
    pub font_size: Pixels,
    pub font: Font,
    pub transform_text: Option<TransformTextFn>,
    pub cursor_visible: bool,
}

pub(crate) struct PrepaintState {
    pub line: Option<ShapedLine>,
    pub cursor: Option<PaintQuad>,
    pub selection: Option<PaintQuad>,
    pub scroll_offset: Pixels,
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
        let content = self.input.read(cx).value();
        let selected_range = self.input.read(cx).selected_range.clone();
        let cursor = self.input.read(cx).cursor_offset();
        let marked_range = self.input.read(cx).marked_range.clone();

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
            font: self.font.clone(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if let Some(marked_range) = marked_range.as_ref() {
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

        let line = window
            .text_system()
            .shape_line(display_text, self.font_size, &runs, None);

        let cursor_x = line.x_for_index(cursor);
        let container_width = bounds.size.width;
        let text_width = line.width;

        let scroll_offset = self.input.update(cx, |input, _cx| {
            input.last_text_width = text_width;
            input.last_bounds = Some(bounds);
            input.ensure_cursor_visible_horizontal(cursor_x, container_width)
        });

        let (selection, cursor_quad) = if selected_range.is_empty() {
            (
                None,
                Some(make_cursor_quad(
                    bounds,
                    cursor_x,
                    scroll_offset,
                    self.text_color,
                )),
            )
        } else {
            let selection_start_x = line.x_for_index(selected_range.start);
            let selection_end_x = line.x_for_index(selected_range.end);
            (
                Some(make_selection_quad(
                    bounds,
                    selection_start_x,
                    selection_end_x,
                    scroll_offset,
                    self.highlight_text_color,
                )),
                None,
            )
        };

        PrepaintState {
            line: Some(line),
            cursor: cursor_quad,
            selection,
            scroll_offset,
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

        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            if let Some(selection) = prepaint.selection.take() {
                window.paint_quad(selection)
            }

            let line = prepaint.line.take().unwrap();
            let text_origin = point(bounds.origin.x - prepaint.scroll_offset, bounds.origin.y);
            line.paint(
                text_origin,
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
        });
    }
}

/// Element for rendering a single line in a multi-line input (non-wrapped mode).
pub(crate) struct LineElement {
    pub input: Entity<InputState>,
    pub line_index: usize,
    pub line_start_offset: usize,
    pub line_end_offset: usize,
    pub text_color: Hsla,
    pub placeholder_text_color: Hsla,
    pub highlight_text_color: Hsla,
    pub line_height: Pixels,
    pub font_size: Pixels,
    pub font: Font,
    pub transform_text: Option<TransformTextFn>,
    pub cursor_visible: bool,
    pub selected_range: Range<usize>,
    pub cursor_offset: usize,
    pub placeholder: SharedString,
    pub is_empty: bool,
}

pub(crate) struct LinePrepaintState {
    pub line: Option<ShapedLine>,
    pub cursor: Option<PaintQuad>,
    pub selection: Option<PaintQuad>,
    pub scroll_offset: Pixels,
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

        let line_content: String =
            full_value[self.line_start_offset..self.line_end_offset].to_string();

        let (display_text, text_color): (SharedString, Hsla) =
            if self.is_empty && self.line_index == 0 {
                (self.placeholder.clone(), self.placeholder_text_color)
            } else if let Some(transform) = &self.transform_text {
                let transformed: String = line_content.chars().map(|c| transform(c)).collect();
                (transformed.into(), self.text_color)
            } else {
                (line_content.clone().into(), self.text_color)
            };

        let run = TextRun {
            len: display_text.len(),
            font: self.font.clone(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let line = window
            .text_system()
            .shape_line(display_text, self.font_size, &[run], None);

        let cursor_on_this_line = self.cursor_offset >= self.line_start_offset
            && self.cursor_offset <= self.line_end_offset;

        let local_cursor = if cursor_on_this_line {
            Some(self.cursor_offset - self.line_start_offset)
        } else {
            None
        };

        let selection_intersects = self.selected_range.start <= self.line_end_offset
            && self.selected_range.end > self.line_start_offset;

        let container_width = bounds.size.width;
        let scroll_offset = {
            let cursor_x = if let Some(local_cursor) = local_cursor {
                line.x_for_index(local_cursor)
            } else {
                let cursor_line = self.input.read(cx).offset_to_line_col(self.cursor_offset).0;
                if cursor_line == self.line_index {
                    line.x_for_index(0)
                } else {
                    let input = self.input.read(cx);
                    let (cursor_line_idx, cursor_col) =
                        input.offset_to_line_col(self.cursor_offset);
                    let cursor_line_start = input.line_start_offset(cursor_line_idx);
                    let cursor_line_end = input.line_end_offset(cursor_line_idx);
                    let cursor_line_content: String =
                        input.value()[cursor_line_start..cursor_line_end].to_string();

                    let cursor_run = TextRun {
                        len: cursor_line_content.len(),
                        font: self.font.clone(),
                        color: self.text_color,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    };
                    let cursor_line_shaped = window.text_system().shape_line(
                        cursor_line_content.into(),
                        self.font_size,
                        &[cursor_run],
                        None,
                    );
                    cursor_line_shaped.x_for_index(cursor_col)
                }
            };

            let text_width = line.width;
            self.input.update(cx, |input, _cx| {
                if text_width > input.last_text_width {
                    input.last_text_width = text_width;
                }
                input.last_bounds = Some(bounds);
                input.ensure_cursor_visible_horizontal(cursor_x, container_width)
            })
        };

        let (selection, cursor) = if !self.selected_range.is_empty() && selection_intersects {
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

            let selection_start_x = line.x_for_index(local_start);
            let mut selection_end_x = line.x_for_index(local_end);

            if should_show_trailing_whitespace(
                &self.selected_range,
                self.line_end_offset,
                line_content.len(),
                local_end,
                &full_value,
            ) {
                let space_run = TextRun {
                    len: 1,
                    font: self.font.clone(),
                    color: self.text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let space_line =
                    window
                        .text_system()
                        .shape_line(" ".into(), self.font_size, &[space_run], None);
                selection_end_x = selection_end_x + space_line.x_for_index(1);
            }

            (
                Some(make_selection_quad(
                    bounds,
                    selection_start_x,
                    selection_end_x,
                    scroll_offset,
                    self.highlight_text_color,
                )),
                None,
            )
        } else if let Some(local_cursor) = local_cursor {
            let cursor_pos = line.x_for_index(local_cursor);
            (
                None,
                Some(make_cursor_quad(
                    bounds,
                    cursor_pos,
                    scroll_offset,
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
            scroll_offset,
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
        let line = prepaint.line.take().unwrap();

        self.input.update(cx, |input, _cx| {
            input.visible_lines_info.push(VisibleLineInfo {
                line_index: self.line_index,
                bounds,
                shaped_line: line.clone(),
            });
        });

        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            if let Some(selection) = prepaint.selection.take() {
                window.paint_quad(selection)
            }

            let text_origin = point(bounds.origin.x - prepaint.scroll_offset, bounds.origin.y);
            line.paint(
                text_origin,
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
        });
    }
}

/// Element for rendering a single visual line in wrapped multi-line input.
pub(crate) struct WrappedLineElement {
    pub input: Entity<InputState>,
    pub visual_line_index: usize,
    pub text_color: Hsla,
    pub placeholder_text_color: Hsla,
    pub highlight_text_color: Hsla,
    pub line_height: Pixels,
    pub font_size: Pixels,
    pub font: Font,
    pub transform_text: Option<TransformTextFn>,
    pub cursor_visible: bool,
    pub selected_range: Range<usize>,
    pub cursor_offset: usize,
    pub placeholder: SharedString,
    pub is_empty: bool,
}

pub(crate) struct WrappedLinePrepaintState {
    pub line: Option<ShapedLine>,
    pub cursor: Option<PaintQuad>,
    pub selection: Option<PaintQuad>,
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
        let actual_line_width = bounds.size.width;
        {
            let input = self.input.read(cx);
            if let Some(precomputed_width) = input.precomputed_at_width {
                if (precomputed_width - actual_line_width).abs() > WRAP_WIDTH_EPSILON
                    && !input.needs_wrap_recompute
                {
                    let _ = input;
                    self.input.update(cx, |input, cx| {
                        input.cached_wrap_width = Some(actual_line_width);
                        input.needs_wrap_recompute = true;
                        cx.notify();
                    });
                }
            }
        }

        let input = self.input.read(cx);

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

        let (display_text, text_color): (SharedString, Hsla) =
            if self.is_empty && self.visual_line_index == 0 {
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
            font: self.font.clone(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let line = window
            .text_system()
            .shape_line(display_text, self.font_size, &[run], None);

        let line_start = info.start_offset;
        let line_end = info.end_offset;
        let line_len = line_end - line_start;

        let cursor_on_this_line =
            self.cursor_offset >= line_start && self.cursor_offset <= line_end;

        let local_cursor = if cursor_on_this_line {
            Some(self.cursor_offset - line_start)
        } else {
            None
        };

        let selection_intersects =
            self.selected_range.start <= line_end && self.selected_range.end > line_start;

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

            let selection_start_x = line.x_for_index(local_start);
            let mut selection_end_x = line.x_for_index(local_end);

            let value = input.value();
            if should_show_trailing_whitespace(
                &self.selected_range,
                line_end,
                line_len,
                local_end,
                &value,
            ) {
                let space_run = TextRun {
                    len: 1,
                    font: self.font.clone(),
                    color: self.text_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let space_line =
                    window
                        .text_system()
                        .shape_line(" ".into(), self.font_size, &[space_run], None);
                selection_end_x = selection_end_x + space_line.x_for_index(1);
            }

            (
                Some(make_selection_quad(
                    bounds,
                    selection_start_x,
                    selection_end_x,
                    Pixels::ZERO,
                    self.highlight_text_color,
                )),
                None,
            )
        } else if let Some(local_cursor) = local_cursor {
            let cursor_pos = line.x_for_index(local_cursor);
            (
                None,
                Some(make_cursor_quad(
                    bounds,
                    cursor_pos,
                    Pixels::ZERO,
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
pub(crate) struct UniformListInputElement {
    pub input: Entity<InputState>,
    pub child: AnyElement,
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

        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

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

        self.input.update(cx, |input, _cx| {
            input.visible_lines_info.clear();
        });

        self.child.paint(window, cx);

        self.input.update(cx, |input, cx| {
            input.last_bounds = Some(bounds);
            cx.notify();
        });
    }
}
