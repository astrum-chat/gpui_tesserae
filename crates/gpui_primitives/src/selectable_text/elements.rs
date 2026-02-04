use std::ops::Range;

use gpui::{
    AnyElement, App, Bounds, DispatchPhase, Element, ElementId, Entity, Font, GlobalElementId,
    Hsla, InspectorElementId, IntoElement, LayoutId, MouseMoveEvent, PaintQuad, Pixels, ShapedLine,
    SharedString, Style, TextRun, Window, point, relative,
};

use crate::selectable_text::VisibleLineInfo;
use crate::selectable_text::state::SelectableTextState;
use crate::utils::{WRAP_WIDTH_EPSILON, make_selection_quad, should_show_trailing_whitespace};

fn create_text_run(font: Font, color: Hsla, len: usize) -> TextRun {
    TextRun {
        len,
        font,
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

fn compute_selection_quad(
    line: &ShapedLine,
    bounds: Bounds<Pixels>,
    selected_range: &Range<usize>,
    line_start: usize,
    line_end: usize,
    is_select_all: bool,
    font: &Font,
    font_size: Pixels,
    text_color: Hsla,
    highlight_color: Hsla,
    window: &mut Window,
) -> Option<PaintQuad> {
    let selection_intersects = selected_range.start <= line_end && selected_range.end >= line_start;

    if selected_range.is_empty() || !selection_intersects {
        return None;
    }

    let line_len = line_end - line_start;
    let local_start = selected_range
        .start
        .saturating_sub(line_start)
        .min(line_len);
    let local_end = selected_range.end.saturating_sub(line_start).min(line_len);

    let selection_start_x = line.x_for_index(local_start);
    let mut selection_end_x = line.x_for_index(local_end);

    if should_show_trailing_whitespace(selected_range, line_end, is_select_all) {
        let space_run = create_text_run(font.clone(), text_color, 1);
        let space_line = window
            .text_system()
            .shape_line(" ".into(), font_size, &[space_run], None);
        selection_end_x = selection_end_x + space_line.x_for_index(1);
    }

    Some(make_selection_quad(
        bounds,
        selection_start_x,
        selection_end_x,
        Pixels::ZERO,
        highlight_color,
    ))
}

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
        let display_text: SharedString = line_content.into();

        let run = create_text_run(self.font.clone(), self.text_color, display_text.len());
        let line = window
            .text_system()
            .shape_line(display_text, self.font_size, &[run], None);

        if self.measured_width.is_none() {
            let line_width = line.width;
            self.state.update(cx, |state, cx| {
                let current_max = state.measured_max_line_width.unwrap_or(Pixels::ZERO);
                if line_width > current_max {
                    state.measured_max_line_width = Some(line_width);
                    cx.notify();
                }
            });
        }

        let selection = compute_selection_quad(
            &line,
            bounds,
            &self.selected_range,
            self.line_start_offset,
            self.line_end_offset,
            self.is_select_all,
            &self.font,
            self.font_size,
            self.text_color,
            self.highlight_text_color,
            window,
        );

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

        let run = create_text_run(self.font.clone(), self.text_color, display_text.len());
        let line = window
            .text_system()
            .shape_line(display_text, self.font_size, &[run], None);

        let selection = compute_selection_quad(
            &line,
            bounds,
            &self.selected_range,
            info.start_offset,
            info.end_offset,
            self.is_select_all,
            &self.font,
            self.font_size,
            self.text_color,
            self.highlight_text_color,
            window,
        );

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

pub(crate) struct UniformListElement {
    pub state: Entity<SelectableTextState>,
    pub child: AnyElement,
    pub debug_wrapping: bool,
}

impl UniformListElement {
    fn should_recompute_wrapping(&self, bounds: Bounds<Pixels>, cx: &App) -> bool {
        let state = self.state.read(cx);
        state.precomputed_at_width.map_or(false, |pw| {
            (bounds.size.width - pw).abs() > WRAP_WIDTH_EPSILON
        })
    }

    fn recompute_wrapping_if_needed(
        &self,
        bounds: Bounds<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) {
        if !self.should_recompute_wrapping(bounds, cx) {
            return;
        }

        if let (Some(font), Some(font_size), Some(text_color)) = {
            let state = self.state.read(cx);
            (
                state.last_font.clone(),
                state.last_font_size,
                state.last_text_color,
            )
        } {
            self.state.update(cx, |state, cx| {
                state.precompute_wrapped_lines(
                    bounds.size.width,
                    font_size,
                    font,
                    text_color,
                    window,
                );
                cx.notify();
            });
        }
    }

    fn paint_debug_overlays(&self, bounds: Bounds<Pixels>, window: &mut Window, cx: &mut App) {
        if !self.debug_wrapping {
            return;
        }

        let precomputed_at_width = self.state.read(cx).precomputed_at_width;
        if let Some(wrap_width) = precomputed_at_width {
            let debug_bounds = Bounds {
                origin: bounds.origin,
                size: gpui::Size {
                    width: wrap_width,
                    height: bounds.size.height,
                },
            };
            window.paint_quad(gpui::PaintQuad {
                bounds: debug_bounds,
                corner_radii: gpui::Corners::default(),
                background: gpui::Hsla {
                    h: 0.0,
                    s: 1.0,
                    l: 0.5,
                    a: 0.2,
                }
                .into(),
                border_widths: gpui::Edges::default(),
                border_color: gpui::Hsla::transparent_black(),
                border_style: gpui::BorderStyle::default(),
            });
        }

        let actual_bounds = Bounds {
            origin: bounds.origin,
            size: gpui::Size {
                width: bounds.size.width,
                height: bounds.size.height,
            },
        };
        window.paint_quad(gpui::PaintQuad {
            bounds: actual_bounds,
            corner_radii: gpui::Corners::default(),
            background: gpui::Hsla::transparent_black().into(),
            border_widths: gpui::Edges::all(Pixels::from(2.0)),
            border_color: gpui::Hsla {
                h: 0.33,
                s: 1.0,
                l: 0.5,
                a: 0.8,
            },
            border_style: gpui::BorderStyle::default(),
        });
    }

    /// Checks if the container width changed and triggers a wrap recompute if needed.
    fn check_container_width_change(&self, actual_width: Pixels, cx: &mut App) {
        if actual_width <= Pixels::ZERO {
            return;
        }

        let (precomputed_at_width, cached_wrap_width, needs_wrap_recompute) = {
            let state = self.state.read(cx);
            (
                state.precomputed_at_width,
                state.cached_wrap_width,
                state.needs_wrap_recompute,
            )
        };

        if cached_wrap_width.is_none() {
            self.initialize_wrap_width(actual_width, precomputed_at_width, cx);
            return;
        }

        if needs_wrap_recompute {
            return;
        }

        if let Some(precomputed_width) = precomputed_at_width {
            if (actual_width - precomputed_width).abs() > WRAP_WIDTH_EPSILON {
                self.trigger_wrap_recompute(actual_width, cx);
            }
        }
    }

    fn initialize_wrap_width(
        &self,
        actual_width: Pixels,
        precomputed_at_width: Option<Pixels>,
        cx: &mut App,
    ) {
        self.state.update(cx, |state, cx| {
            state.cached_wrap_width = Some(actual_width);
            let needs_recompute = precomputed_at_width
                .map(|pw| (actual_width - pw).abs() > WRAP_WIDTH_EPSILON)
                .unwrap_or(true);
            if needs_recompute {
                state.needs_wrap_recompute = true;
                cx.notify();
            }
        });
    }

    fn trigger_wrap_recompute(&self, actual_width: Pixels, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            state.cached_wrap_width = Some(actual_width);
            state.needs_wrap_recompute = true;
            cx.notify();
        });
    }
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
        (self.child.request_layout(window, cx), ())
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
        // Check and update wrap width BEFORE child prepaint so the child uses current width
        self.check_container_width_change(bounds.size.width, cx);
        self.recompute_wrapping_if_needed(bounds, window, cx);

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
        self.paint_debug_overlays(bounds, window, cx);

        self.state.update(cx, |state, _cx| {
            state.last_bounds = Some(bounds);
        });
    }
}
