use std::ops::Range;

use gpui::{
    AnyElement, App, Bounds, DispatchPhase, Element, ElementId, Entity, Font, GlobalElementId,
    Hsla, InspectorElementId, IntoElement, LayoutId, MouseMoveEvent, PaintQuad, Pixels, ShapedLine,
    SharedString, Style, Window, point, px, relative, size,
};

use crate::extensions::WindowExt;
use crate::selectable_text::VisibleLineInfo;
use crate::selectable_text::state::SelectableTextState;
use crate::utils::{
    SelectionShape, WIDTH_WRAP_BASE_MARGIN, compute_selection_shape, compute_selection_x_bounds,
    create_text_run,
};

/// Paints alternating colored rectangles for each character's measured bounds.
fn paint_character_bounds(
    line: &ShapedLine,
    bounds: Bounds<Pixels>,
    char_count: usize,
    window: &mut Window,
) {
    let colors = [
        Hsla {
            h: 0.0,
            s: 1.0,
            l: 0.5,
            a: 0.15,
        }, // red
        Hsla {
            h: 0.6,
            s: 1.0,
            l: 0.5,
            a: 0.15,
        }, // blue
    ];
    let border_color = Hsla {
        h: 0.0,
        s: 0.0,
        l: 1.0,
        a: 0.3,
    };

    for i in 0..char_count {
        let x_start = line.x_for_index(i);
        let x_end = line.x_for_index(i + 1);

        let char_bounds = Bounds::new(
            point(bounds.origin.x + x_start, bounds.origin.y),
            size(x_end - x_start, bounds.size.height),
        );

        window.paint_quad(PaintQuad {
            bounds: char_bounds,
            corner_radii: gpui::Corners::default(),
            background: colors[i % 2].into(),
            border_widths: gpui::Edges::all(px(0.5)),
            border_color,
            border_style: gpui::BorderStyle::default(),
        });
    }
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
    pub measured_width: Option<Pixels>,
    pub selection_rounded: Option<Pixels>,
    pub selection_rounded_smoothing: Option<f32>,
    pub prev_line_offsets: Option<(usize, usize)>,
    pub next_line_offsets: Option<(usize, usize)>,
    pub debug_character_bounds: bool,
}

pub(crate) struct LinePrepaintState {
    pub line: Option<ShapedLine>,
    pub selection: Option<SelectionShape>,
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
            let line_width = window.round(line.width);
            self.state.update(cx, |state, cx| {
                let current_max = state.measured_max_line_width.unwrap_or(Pixels::ZERO);
                if line_width > current_max {
                    state.measured_max_line_width = Some(line_width);
                    cx.notify();
                }
            });
        }

        // Compute adjacent line selection bounds for corner radius calculation
        let (prev_line_bounds, next_line_bounds) = if self.selection_rounded.is_some() {
            let prev_bounds = self.prev_line_offsets.and_then(|(start, end)| {
                let content: String = full_value[start..end].to_string();
                let run = create_text_run(self.font.clone(), self.text_color, content.len());
                let shaped =
                    window
                        .text_system()
                        .shape_line(content.into(), self.font_size, &[run], None);
                compute_selection_x_bounds(
                    &shaped,
                    &self.selected_range,
                    start,
                    end,
                    &self.font,
                    self.font_size,
                    self.text_color,
                    window,
                )
            });

            let next_bounds = self.next_line_offsets.and_then(|(start, end)| {
                let content: String = full_value[start..end].to_string();
                let run = create_text_run(self.font.clone(), self.text_color, content.len());
                let shaped =
                    window
                        .text_system()
                        .shape_line(content.into(), self.font_size, &[run], None);
                compute_selection_x_bounds(
                    &shaped,
                    &self.selected_range,
                    start,
                    end,
                    &self.font,
                    self.font_size,
                    self.text_color,
                    window,
                )
            });

            (prev_bounds, next_bounds)
        } else {
            (None, None)
        };

        let selection = compute_selection_shape(
            &line,
            bounds,
            &self.selected_range,
            self.line_start_offset,
            self.line_end_offset,
            &self.font,
            self.font_size,
            self.text_color,
            self.highlight_text_color,
            Pixels::ZERO,
            window,
            self.selection_rounded,
            self.selection_rounded_smoothing,
            prev_line_bounds,
            next_line_bounds,
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

        let debug_chars = self.debug_character_bounds;
        let char_count = self.line_end_offset - self.line_start_offset;

        window.with_content_mask(Some(gpui::ContentMask { bounds }), |window| {
            if debug_chars {
                paint_character_bounds(&line, bounds, char_count, window);
            }

            if let Some(selection) = prepaint.selection.take() {
                selection.paint(window);
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
    pub selection_rounded: Option<Pixels>,
    pub selection_rounded_smoothing: Option<f32>,
    pub prev_visual_line_offsets: Option<(usize, usize)>,
    pub next_visual_line_offsets: Option<(usize, usize)>,
    pub debug_character_bounds: bool,
}

pub(crate) struct WrappedLinePrepaintState {
    pub line: Option<ShapedLine>,
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

        // Compute adjacent line selection bounds for corner radius calculation
        let (prev_line_bounds, next_line_bounds) = if self.selection_rounded.is_some() {
            let prev_bounds = self.prev_visual_line_offsets.and_then(|(start, end)| {
                let content: String = value[start..end].to_string();
                let run = create_text_run(self.font.clone(), self.text_color, content.len());
                let shaped =
                    window
                        .text_system()
                        .shape_line(content.into(), self.font_size, &[run], None);
                compute_selection_x_bounds(
                    &shaped,
                    &self.selected_range,
                    start,
                    end,
                    &self.font,
                    self.font_size,
                    self.text_color,
                    window,
                )
            });

            let next_bounds = self.next_visual_line_offsets.and_then(|(start, end)| {
                let content: String = value[start..end].to_string();
                let run = create_text_run(self.font.clone(), self.text_color, content.len());
                let shaped =
                    window
                        .text_system()
                        .shape_line(content.into(), self.font_size, &[run], None);
                compute_selection_x_bounds(
                    &shaped,
                    &self.selected_range,
                    start,
                    end,
                    &self.font,
                    self.font_size,
                    self.text_color,
                    window,
                )
            });

            (prev_bounds, next_bounds)
        } else {
            (None, None)
        };

        let selection = compute_selection_shape(
            &line,
            bounds,
            &self.selected_range,
            info.start_offset,
            info.end_offset,
            &self.font,
            self.font_size,
            self.text_color,
            self.highlight_text_color,
            Pixels::ZERO,
            window,
            self.selection_rounded,
            self.selection_rounded_smoothing,
            prev_line_bounds,
            next_line_bounds,
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
        let Some(line) = prepaint.line.take() else {
            return;
        };

        if self.debug_character_bounds {
            let char_count = self
                .state
                .read(cx)
                .precomputed_visual_lines
                .get(self.visual_line_index)
                .map(|info| info.end_offset - info.start_offset)
                .unwrap_or(0);
            paint_character_bounds(&line, bounds, char_count, window);
        }

        if let Some(selection) = prepaint.selection.take() {
            selection.paint(window);
        }

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
            if (actual_width - precomputed_width).abs() > WIDTH_WRAP_BASE_MARGIN {
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
                .map(|pw| (actual_width - pw).abs() > WIDTH_WRAP_BASE_MARGIN)
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
        // Update cached_wrap_width and flag recompute if the container width changed.
        // Do NOT immediately recompute here — the uniform_list was already created with
        // a fixed item count during render. Reshaping at a narrower width mid-frame could
        // produce more visual lines than the list has slots for, causing words to disappear.
        // Instead, let check_container_width_change set needs_wrap_recompute + cx.notify()
        // so the next frame's render reshapes with the correct cached_wrap_width.
        self.check_container_width_change(bounds.size.width, cx);

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
                    // Store position for deferred processing after visible_lines_info is populated
                    state.pending_selection_position = Some(event.position);
                    cx.notify();
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

        // Process pending selection after visible_lines_info is fully populated
        self.state.update(cx, |state, cx| {
            state.process_pending_selection(cx);
        });
    }
}
