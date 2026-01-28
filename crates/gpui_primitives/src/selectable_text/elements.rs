use std::ops::Range;

use gpui::{
    AnyElement, App, Bounds, DispatchPhase, Element, ElementId, Entity, Font, GlobalElementId,
    Hsla, InspectorElementId, IntoElement, LayoutId, MouseMoveEvent, PaintQuad, Pixels, ShapedLine,
    SharedString, Style, TextRun, Window, point, relative,
};

use crate::selectable_text::VisibleLineInfo;
use crate::selectable_text::state::SelectableTextState;
use crate::utils::{WRAP_WIDTH_EPSILON, make_selection_quad, should_show_trailing_whitespace};

/// Renders one logical line in non-wrapped multiline mode.
pub(crate) struct LineElement {
    pub state: Entity<SelectableTextState>,
    pub line_index: usize,
    pub line_start_offset: usize,
    pub line_end_offset: usize,
    pub text_color: Hsla,
    pub highlight_text_color: Hsla,
    pub line_height: Pixels,
    pub font_size: Pixels,
    pub font: Font,
    pub selected_range: Range<usize>,
    pub is_select_all: bool,
    /// Measured width for w_auto support (None means use relative(1.))
    pub measured_width: Option<Pixels>,
}

pub(crate) struct LinePrepaintState {
    pub line: Option<ShapedLine>,
    pub selection: Option<PaintQuad>,
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
        // Always fill parent width so prepaint can detect actual available width.
        // This allows the clamping detection to work when the parent resizes.
        // The text content will still render at the correct wrapped positions.
        style.size.width = relative(1.).into();
        style.size.height = self.line_height.into();

        (window.request_layout(style, [], cx), ())
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
        let state = self.state.read(cx);
        let full_value = state.get_text();

        let line_content: String =
            full_value[self.line_start_offset..self.line_end_offset].to_string();

        let display_text: SharedString = line_content.clone().into();

        let run = TextRun {
            len: display_text.len(),
            font: self.font.clone(),
            color: self.text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let line = window
            .text_system()
            .shape_line(display_text, self.font_size, &[run], None);

        // Measure line width for w_auto support (only if not already using measured width)
        if self.measured_width.is_none() {
            let line_width = line.width;
            self.state.update(cx, |state, cx| {
                let current_max = state.measured_max_line_width.unwrap_or(Pixels::ZERO);
                if line_width > current_max {
                    state.measured_max_line_width = Some(line_width);
                    cx.notify(); // Trigger re-render with new width
                }
            });
        }

        let selection_intersects = self.selected_range.start <= self.line_end_offset
            && self.selected_range.end >= self.line_start_offset;

        let selection = if !self.selected_range.is_empty() && selection_intersects {
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
                self.is_select_all,
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

            Some(make_selection_quad(
                bounds,
                selection_start_x,
                selection_end_x,
                Pixels::ZERO,
                self.highlight_text_color,
            ))
        } else {
            None
        };

        LinePrepaintState {
            line: Some(line),
            selection,
        }
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
        let line = prepaint.line.take().unwrap();

        self.state.update(cx, |state, _cx| {
            state.visible_lines_info.push(VisibleLineInfo {
                line_index: self.line_index,
                bounds,
                shaped_line: line.clone(),
            });
        });

        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            if let Some(selection) = prepaint.selection.take() {
                window.paint_quad(selection)
            }

            let text_origin = point(bounds.origin.x, bounds.origin.y);
            line.paint(
                text_origin,
                self.line_height,
                gpui::TextAlign::Left,
                None,
                window,
                cx,
            )
            .unwrap();
        });
    }
}

/// Renders one visual line segment in wrapped mode.
pub(crate) struct WrappedLineElement {
    pub state: Entity<SelectableTextState>,
    pub visual_line_index: usize,
    pub text_color: Hsla,
    pub highlight_text_color: Hsla,
    pub line_height: Pixels,
    pub font_size: Pixels,
    pub font: Font,
    pub selected_range: Range<usize>,
    pub is_select_all: bool,
}

pub(crate) struct WrappedLinePrepaintState {
    pub line: Option<ShapedLine>,
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
        // Always fill parent width so prepaint can detect actual available width.
        // This allows the clamping detection to work when the parent resizes.
        // The text content will still render at the correct wrapped positions.
        style.size.width = relative(1.).into();
        style.size.height = self.line_height.into();

        (window.request_layout(style, [], cx), ())
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
        let actual_line_width = bounds.size.width;
        {
            let state = self.state.read(cx);
            if let Some(precomputed_width) = state.precomputed_at_width {
                // Update cached_wrap_width if:
                // 1. We were CLAMPED (actual < what we asked for) - max_size kicked in
                // 2. Width INCREASED (actual > precomputed) - parent grew, we should re-wrap
                let was_clamped = actual_line_width < precomputed_width - WRAP_WIDTH_EPSILON;
                let width_increased = actual_line_width > precomputed_width + WRAP_WIDTH_EPSILON;
                if (was_clamped || width_increased) && !state.needs_wrap_recompute {
                    let _ = state;
                    self.state.update(cx, |state, cx| {
                        state.cached_wrap_width = Some(actual_line_width);
                        state.needs_wrap_recompute = true;
                        cx.notify();
                    });
                }
            }
        }

        let state = self.state.read(cx);

        let visual_info = state
            .precomputed_visual_lines
            .get(self.visual_line_index)
            .cloned();

        let Some(info) = visual_info else {
            return WrappedLinePrepaintState {
                line: None,
                selection: None,
            };
        };

        let value = state.get_text();
        let segment = &value[info.start_offset..info.end_offset];
        let display_text: SharedString = segment.to_string().into();

        let run = TextRun {
            len: display_text.len(),
            font: self.font.clone(),
            color: self.text_color,
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

        let selection_intersects =
            self.selected_range.start <= line_end && self.selected_range.end >= line_start;

        let selection = if !self.selected_range.is_empty() && selection_intersects {
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

            if should_show_trailing_whitespace(&self.selected_range, line_end, self.is_select_all) {
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

            Some(make_selection_quad(
                bounds,
                selection_start_x,
                selection_end_x,
                Pixels::ZERO,
                self.highlight_text_color,
            ))
        } else {
            None
        };

        WrappedLinePrepaintState {
            line: Some(line),
            selection,
        }
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
        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }

        let Some(line) = prepaint.line.take() else {
            return;
        };

        self.state.update(cx, |state, _cx| {
            state.visible_lines_info.push(VisibleLineInfo {
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
    }
}

/// Coordinates the uniform_list with selection handling.
pub(crate) struct UniformListElement {
    pub state: Entity<SelectableTextState>,
    pub child: AnyElement,
}

impl IntoElement for UniformListElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for UniformListElement {
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
        let state = self.state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
            if phase == DispatchPhase::Capture {
                return;
            }

            state.update(cx, |state, cx| {
                if state.is_selecting {
                    if let Some(line_height) = state.line_height {
                        state.select_to_multiline(event.position, line_height, cx);
                    }
                }
            });
        });

        self.state.update(cx, |state, _cx| {
            state.visible_lines_info.clear();
        });

        self.child.paint(window, cx);

        self.state.update(cx, |state, cx| {
            state.last_bounds = Some(bounds);
            cx.notify();
        });
    }
}
