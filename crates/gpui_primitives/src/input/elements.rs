use std::ops::Range;

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, CursorStyle, DispatchPhase, Element, ElementId,
    ElementInputHandler, Entity, Font, GlobalElementId, Hitbox, HitboxBehavior, Hsla,
    InspectorElementId, IntoElement, LayoutId, MouseMoveEvent, PaintQuad, Pixels, ShapedLine,
    SharedString, Style, TextRun, UnderlineStyle, Window, point, px, size,
};

use crate::extensions::WindowExt;
use crate::input::state::InputState;
use crate::input::{TransformTextFn, VisibleLineInfo};
use crate::utils::{
    SelectionShape, TextNavigation, build_selection_shape, compute_selection_shape,
    create_text_run, make_cursor_quad, request_line_layout, selection_config_from_options,
};
use crate::utils::{WIDTH_WRAP_BASE_MARGIN, multiline_height};

fn resolve_display_text(
    content: &str,
    is_placeholder_line: bool,
    placeholder: &SharedString,
    placeholder_color: Hsla,
    text_color: Hsla,
    transform: Option<&TransformTextFn>,
) -> (SharedString, Hsla) {
    if content.is_empty() && is_placeholder_line {
        (placeholder.clone(), placeholder_color)
    } else if content.is_empty() {
        (SharedString::default(), text_color)
    } else if let Some(transform) = transform {
        let transformed: String = content.chars().map(|c| transform(c)).collect();
        (transformed.into(), text_color)
    } else {
        (content.to_string().into(), text_color)
    }
}

/// Handles text shaping, cursor positioning, selection rendering, and horizontal scrolling for single-line inputs.
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
    pub selection_rounded: Option<Pixels>,
    pub selection_rounded_smoothing: Option<f32>,
    pub selection_precise: bool,
}

pub(crate) struct PrepaintState {
    pub line: Option<ShapedLine>,
    pub cursor: Option<PaintQuad>,
    pub selection: Option<SelectionShape>,
    pub scroll_offset: Pixels,
    pub container_hitbox: Option<Hitbox>,
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
        request_line_layout(self.line_height, window, cx)
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

        let (display_text, text_color) = resolve_display_text(
            &content,
            true,
            &self.placeholder,
            self.placeholder_text_color,
            self.text_color,
            self.transform_text.as_ref(),
        );

        let run = create_text_run(self.font.clone(), text_color, display_text.len());

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
            if input.scroll_to_cursor_horizontal {
                input.scroll_to_cursor_horizontal = false;
                input.ensure_cursor_visible_horizontal(cursor_x, container_width)
            } else {
                input.horizontal_scroll_offset
            }
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
            let selection_start_x = window.round(line.x_for_index(selected_range.start));
            let mut selection_end_x = window.round(line.x_for_index(selected_range.end));
            if !self.selection_precise && selected_range.end >= line.len() {
                selection_end_x = line.width.max(bounds.size.width);
            }

            let config = selection_config_from_options(
                self.selection_rounded,
                self.selection_rounded_smoothing,
            );
            let corners = gpui::Corners::all(config.corner_radius);

            let selection_shape = build_selection_shape(
                bounds,
                selection_start_x,
                selection_end_x,
                scroll_offset,
                self.highlight_text_color,
                &config,
                corners,
            );
            (Some(selection_shape), None)
        };

        let container_hitbox = Some(window.insert_hitbox(bounds, HitboxBehavior::Normal));

        PrepaintState {
            line: Some(line),
            cursor: cursor_quad,
            selection,
            scroll_offset,
            container_hitbox,
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

        if self.input.read(cx).is_selecting {
            if let Some(hitbox) = &prepaint.container_hitbox {
                window.set_cursor_style(CursorStyle::IBeam, hitbox);
            }
        }

        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            if let Some(selection) = prepaint.selection.take() {
                selection.paint(window);
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

/// Renders one logical line in non-wrapped multiline mode. Handles per-line cursor, selection, and horizontal scroll offset.
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
    pub selection_rounded: Option<Pixels>,
    pub selection_rounded_smoothing: Option<f32>,
    pub prev_line_offsets: Option<(usize, usize)>,
    pub next_line_offsets: Option<(usize, usize)>,
    pub selection_precise: bool,
    pub debug_interior_corners: bool,
}

pub(crate) struct LinePrepaintState {
    pub line: Option<ShapedLine>,
    pub cursor: Option<PaintQuad>,
    pub selection: Option<SelectionShape>,
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
        request_line_layout(self.line_height, window, cx)
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
        let is_actually_empty = full_value.is_empty();

        // Check bounds before slicing
        if !is_actually_empty
            && (self.line_start_offset > full_value.len()
                || self.line_end_offset > full_value.len())
        {
            return LinePrepaintState {
                line: None,
                cursor: None,
                selection: None,
                scroll_offset: Pixels::ZERO,
            };
        }

        let line_content: String = if is_actually_empty {
            String::new()
        } else {
            full_value[self.line_start_offset..self.line_end_offset].to_string()
        };

        let (display_text, text_color) = resolve_display_text(
            &line_content,
            is_actually_empty && self.line_index == 0,
            &self.placeholder,
            self.placeholder_text_color,
            self.text_color,
            self.transform_text.as_ref(),
        );

        let run = create_text_run(self.font.clone(), text_color, display_text.len());

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

        let selection_intersects = self.selected_range.start < self.line_end_offset
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

                    let cursor_run = create_text_run(
                        self.font.clone(),
                        self.text_color,
                        cursor_line_content.len(),
                    );
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
                if input.scroll_to_cursor_horizontal {
                    input.scroll_to_cursor_horizontal = false;
                    input.ensure_cursor_visible_horizontal(cursor_x, container_width)
                } else {
                    input.horizontal_scroll_offset
                }
            })
        };

        let content_width = Some(self.input.read(cx).last_text_width);

        let (selection, cursor) = if !self.selected_range.is_empty() && selection_intersects {
            let (prev_line_bounds, next_line_bounds) =
                crate::utils::compute_adjacent_line_selection_bounds(
                    &full_value,
                    self.prev_line_offsets,
                    self.next_line_offsets,
                    &self.selected_range,
                    self.selection_rounded,
                    &self.font,
                    self.font_size,
                    self.text_color,
                    window,
                );

            let selection_shape = compute_selection_shape(
                &line,
                bounds,
                &self.selected_range,
                self.line_start_offset,
                self.line_end_offset,
                &self.font,
                self.font_size,
                self.text_color,
                self.highlight_text_color,
                scroll_offset,
                false, // not wrapped — non-wrapped multiline mode
                self.selection_precise,
                content_width,
                window,
                self.selection_rounded,
                self.selection_rounded_smoothing,
                prev_line_bounds,
                self.prev_line_offsets.map(|(_, end)| end),
                next_line_bounds,
                self.next_line_offsets.map(|(_, end)| end),
                self.debug_interior_corners,
            );

            (selection_shape, None)
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

        if let Some(selection) = prepaint.selection.take() {
            selection.paint(window);
        }

        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
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

/// Renders one visual line segment in wrapped mode. A single logical line may span multiple WrappedLineElements when text wraps.
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
    pub selection_rounded: Option<Pixels>,
    pub selection_rounded_smoothing: Option<f32>,
    pub prev_visual_line_offsets: Option<(usize, usize)>,
    pub next_visual_line_offsets: Option<(usize, usize)>,
    pub selection_precise: bool,
    pub debug_interior_corners: bool,
}

pub(crate) struct WrappedLinePrepaintState {
    pub line: Option<ShapedLine>,
    pub cursor: Option<PaintQuad>,
    pub selection: Option<SelectionShape>,
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
        request_line_layout(self.line_height, window, cx)
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

        let value = input.value();
        let is_actually_empty = value.is_empty();

        let (display_text, text_color): (SharedString, Hsla) =
            if is_actually_empty && self.visual_line_index == 0 {
                (self.placeholder.clone(), self.placeholder_text_color)
            } else if is_actually_empty {
                // Value is empty but this isn't the first visual line - nothing to render
                return WrappedLinePrepaintState {
                    line: None,
                    cursor: None,
                    selection: None,
                };
            } else if info.start_offset > value.len() || info.end_offset > value.len() {
                // Offsets are stale/invalid for current value - nothing to render
                return WrappedLinePrepaintState {
                    line: None,
                    cursor: None,
                    selection: None,
                };
            } else {
                let segment = &value[info.start_offset..info.end_offset];
                resolve_display_text(
                    segment,
                    false,
                    &self.placeholder,
                    self.placeholder_text_color,
                    self.text_color,
                    self.transform_text.as_ref(),
                )
            };

        let run = create_text_run(self.font.clone(), text_color, display_text.len());

        let line = window
            .text_system()
            .shape_line(display_text, self.font_size, &[run], None);

        let line_start = info.start_offset;
        let line_end = info.end_offset;
        let cursor_on_this_line =
            self.cursor_offset >= line_start && self.cursor_offset <= line_end;

        let local_cursor = if cursor_on_this_line {
            Some(self.cursor_offset - line_start)
        } else {
            None
        };

        let selection_intersects =
            self.selected_range.start < line_end && self.selected_range.end > line_start;

        let (selection, cursor) = if !self.selected_range.is_empty() && selection_intersects {
            let (prev_line_bounds, next_line_bounds) =
                crate::utils::compute_adjacent_line_selection_bounds(
                    &value,
                    self.prev_visual_line_offsets,
                    self.next_visual_line_offsets,
                    &self.selected_range,
                    self.selection_rounded,
                    &self.font,
                    self.font_size,
                    self.text_color,
                    window,
                );

            let selection_shape = compute_selection_shape(
                &line,
                bounds,
                &self.selected_range,
                line_start,
                line_end,
                &self.font,
                self.font_size,
                self.text_color,
                self.highlight_text_color,
                Pixels::ZERO,
                true, // wrapped mode
                self.selection_precise,
                None,
                window,
                self.selection_rounded,
                self.selection_rounded_smoothing,
                prev_line_bounds,
                self.prev_visual_line_offsets.map(|(_, end)| end),
                next_line_bounds,
                self.next_visual_line_offsets.map(|(_, end)| end),
                self.debug_interior_corners,
            );

            (selection_shape, None)
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
            selection.paint(window);
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

/// Coordinates the uniform_list with input handling: registers the ElementInputHandler, tracks mouse drag selection across lines, and manages visible line info for hit testing.
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
    type PrepaintState = Hitbox;

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
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        // Detect container width changes and defer wrap recompute to next frame.
        // This must happen here (not in WrappedLineElement::prepaint) because the
        // uniform_list was already built with a fixed item count during render.
        // Reshaping mid-frame could produce more/fewer visual lines than slots,
        // causing text to flash in/out of existence.
        let actual_width = bounds.size.width;
        if actual_width > Pixels::ZERO {
            let (precomputed_at_width, cached_wrap_width, needs_wrap_recompute) = {
                let input = self.input.read(cx);
                (
                    input.precomputed_at_width,
                    input.cached_wrap_width,
                    input.needs_wrap_recompute,
                )
            };

            if cached_wrap_width.is_none() {
                self.input.update(cx, |input, cx| {
                    input.cached_wrap_width = Some(actual_width);
                    let needs_recompute = precomputed_at_width
                        .map(|pw| (actual_width - pw).abs() > WIDTH_WRAP_BASE_MARGIN)
                        .unwrap_or(true);
                    if needs_recompute {
                        input.needs_wrap_recompute = true;
                        cx.notify();
                    }
                });
            } else if !needs_wrap_recompute {
                if let Some(precomputed_width) = precomputed_at_width {
                    if (actual_width - precomputed_width).abs() > WIDTH_WRAP_BASE_MARGIN {
                        self.input.update(cx, |input, cx| {
                            input.cached_wrap_width = Some(actual_width);
                            input.needs_wrap_recompute = true;
                            cx.notify();
                        });
                    }
                }
            }
        }

        self.child.prepaint(window, cx);

        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
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

        if self.input.read(cx).is_selecting {
            window.set_cursor_style(CursorStyle::IBeam, prepaint);
        }

        self.input.update(cx, |input, _cx| {
            input.visible_lines_info.clear();
        });

        self.child.paint(window, cx);

        self.input.update(cx, |input, _cx| {
            input.last_bounds = Some(bounds);
        });
    }
}

/// Custom element that uses `request_measured_layout` with the user's actual style
/// (width, max-width, etc.) to be the Taffy leaf node directly. This eliminates the
/// parent-child height mismatch: since this element IS the container, Taffy's
/// measurement directly determines the container's height.
///
/// The measure callback wraps text at the actual available width and returns the
/// correct height in the same frame - no one-frame clipping during rapid resize.
///
/// Children (WrappedLineElements) are created in prepaint and painted in paint,
/// similar to how GPUI's uniform_list manages its items.
pub(crate) struct WrappedTextInputElement {
    pub input: Entity<InputState>,
    pub text_color: Hsla,
    pub placeholder_text_color: Hsla,
    pub highlight_text_color: Hsla,
    pub line_height: Pixels,
    pub font_size: Pixels,
    pub font: Font,
    pub transform_text: Option<TransformTextFn>,
    pub cursor_visible: bool,
    pub placeholder: SharedString,
    pub selection_rounded: Option<Pixels>,
    pub selection_rounded_smoothing: Option<f32>,
    pub selection_precise: bool,
    pub debug_interior_corners: bool,
    pub multiline_clamp: Option<usize>,
    pub scale_factor: f32,
    pub style: Style,
    /// Created during prepaint, painted during paint.
    pub children: Vec<WrappedLineElement>,
}

pub(crate) struct WrappedTextInputPrepaintState {
    child_prepaints: Vec<WrappedLinePrepaintState>,
    container_hitbox: Hitbox,
}

impl IntoElement for WrappedTextInputElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for WrappedTextInputElement {
    type RequestLayoutState = ();
    type PrepaintState = WrappedTextInputPrepaintState;

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
        _cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let state = self.input.clone();
        let line_height = self.line_height;
        let multiline_clamp = self.multiline_clamp;
        let scale_factor = self.scale_factor;
        let font = self.font.clone();
        let font_size = self.font_size;
        let text_color = self.text_color;

        let style = self.style.clone();

        let layout_id = window.request_measured_layout(style, {
            move |known_dimensions, available_space, window, cx| {
                let width = known_dimensions.width.or(match available_space.width {
                    AvailableSpace::Definite(x) => Some(x),
                    _ => None,
                });

                let Some(width) = width else {
                    // No definite width available - use existing visual lines as fallback
                    let count = state.read(cx).precomputed_visual_lines.len().max(1);
                    let visible = multiline_clamp.map_or(1, |c| c.min(count)).max(1);
                    let height = multiline_height(line_height, visible, scale_factor);
                    return size(Pixels::ZERO, height);
                };

                let wrap_width = width + WIDTH_WRAP_BASE_MARGIN;
                let text = state.read(cx).value();

                let (wrapped_lines, visual_lines) = crate::utils::shape_and_build_visual_lines(
                    &text,
                    wrap_width,
                    font_size,
                    font.clone(),
                    text_color,
                    window,
                );

                let visual_line_count = visual_lines.len().max(1);

                let max_line_width = wrapped_lines
                    .iter()
                    .map(|line| line.unwrapped_layout.width)
                    .fold(Pixels::ZERO, |a, b| if b > a { b } else { a });

                state.update(cx, |state, _cx| {
                    state.precomputed_at_width = Some(wrap_width);
                    state.precomputed_visual_lines = visual_lines;
                    state.precomputed_wrapped_lines = wrapped_lines;
                    state.cached_wrap_width = Some(width);
                    state.clamp_vertical_scroll();

                    if state.scroll_to_cursor_on_next_render {
                        state.scroll_to_cursor_on_next_render = false;
                        state.ensure_cursor_visible();
                    }
                });

                let visible_lines = multiline_clamp
                    .map_or(1, |c| c.min(visual_line_count))
                    .max(1);
                let height = multiline_height(line_height, visible_lines, scale_factor);

                // If Taffy gave us a known width (user set explicit w()), use it.
                // Otherwise return intrinsic content width clamped to available space.
                let result_width = if known_dimensions.width.is_some() {
                    width
                } else {
                    let content_width = window.round(max_line_width) + WIDTH_WRAP_BASE_MARGIN;
                    content_width.min(width)
                };
                size(result_width, height)
            }
        });

        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> WrappedTextInputPrepaintState {
        let actual_line_count = self.input.read(cx).precomputed_visual_lines.len().max(1);
        let visual_lines = self.input.read(cx).precomputed_visual_lines.clone();
        let selected_range = self.input.read(cx).selected_range.clone();
        let cursor_offset = self.input.read(cx).cursor_offset();
        let vertical_scroll_offset = self.input.read(cx).vertical_scroll_offset;

        self.children.clear();
        self.children.reserve(actual_line_count);

        for visual_idx in 0..actual_line_count {
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

            self.children.push(WrappedLineElement {
                input: self.input.clone(),
                visual_line_index: visual_idx,
                text_color: self.text_color,
                placeholder_text_color: self.placeholder_text_color,
                highlight_text_color: self.highlight_text_color,
                line_height: self.line_height,
                font_size: self.font_size,
                font: self.font.clone(),
                transform_text: self.transform_text.clone(),
                cursor_visible: self.cursor_visible,
                selected_range: selected_range.clone(),
                cursor_offset,
                placeholder: self.placeholder.clone(),
                selection_rounded: self.selection_rounded,
                selection_rounded_smoothing: self.selection_rounded_smoothing,
                prev_visual_line_offsets,
                next_visual_line_offsets,
                selection_precise: self.selection_precise,
                debug_interior_corners: self.debug_interior_corners,
            });
        }

        // Prepaint children, positioning them at line_height intervals offset by vertical scroll.
        let mut child_prepaints = Vec::with_capacity(actual_line_count);
        for (idx, child) in self.children.iter_mut().enumerate() {
            let child_bounds = Bounds {
                origin: point(
                    bounds.origin.x,
                    bounds.origin.y + self.line_height * idx as f32 - vertical_scroll_offset,
                ),
                size: gpui::Size {
                    width: bounds.size.width,
                    height: self.line_height,
                },
            };

            let prepaint_state = child.prepaint(None, None, child_bounds, &mut (), window, cx);
            child_prepaints.push(prepaint_state);
        }

        let container_hitbox = window.insert_hitbox(bounds, HitboxBehavior::Normal);
        WrappedTextInputPrepaintState {
            child_prepaints,
            container_hitbox,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut WrappedTextInputPrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();

        // Register ElementInputHandler for IME/text input
        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        // Mouse drag selection
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

        if self.input.read(cx).is_selecting {
            window.set_cursor_style(CursorStyle::IBeam, &prepaint.container_hitbox);
        }

        let vertical_scroll_offset = self.input.read(cx).vertical_scroll_offset;

        self.input.update(cx, |input, _cx| {
            input.visible_lines_info.clear();
        });

        // Paint children with content mask for clipping
        let visible_lines = self
            .multiline_clamp
            .map_or(1, |c| c.min(self.children.len()))
            .max(1);
        let clip_bounds = Bounds {
            origin: bounds.origin,
            size: gpui::Size {
                width: bounds.size.width,
                height: multiline_height(self.line_height, visible_lines, self.scale_factor),
            },
        };

        window.with_content_mask(
            Some(gpui::ContentMask {
                bounds: clip_bounds,
            }),
            |window| {
                for (idx, child) in self.children.iter_mut().enumerate() {
                    let child_bounds = Bounds {
                        origin: point(
                            bounds.origin.x,
                            bounds.origin.y + self.line_height * idx as f32
                                - vertical_scroll_offset,
                        ),
                        size: gpui::Size {
                            width: bounds.size.width,
                            height: self.line_height,
                        },
                    };

                    if let Some(child_prepaint) = prepaint.child_prepaints.get_mut(idx) {
                        child.paint(
                            None,
                            None,
                            child_bounds,
                            &mut (),
                            child_prepaint,
                            window,
                            cx,
                        );
                    }
                }
            },
        );

        self.input.update(cx, |input, _cx| {
            input.last_bounds = Some(bounds);
        });
    }
}
