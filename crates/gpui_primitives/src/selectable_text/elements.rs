use std::ops::Range;

use gpui::{
    AnyElement, App, AvailableSpace, Bounds, CursorStyle, DispatchPhase, Element, ElementId,
    Entity, Font, GlobalElementId, Hitbox, HitboxBehavior, Hsla, InspectorElementId, IntoElement,
    LayoutId, MouseMoveEvent, PaintQuad, Pixels, ShapedLine, SharedString, Style, Window, point,
    px, relative, size,
};

use crate::extensions::WindowExt;
use crate::selectable_text::VisibleLineInfo;
use crate::selectable_text::state::SelectableTextState;
use crate::utils::{
    SelectionShape, WIDTH_WRAP_BASE_MARGIN, compute_selection_shape, compute_selection_x_bounds,
    create_text_run, multiline_height,
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
    pub debug_interior_corners: bool,
}

pub(crate) struct LinePrepaintState {
    pub line: Option<ShapedLine>,
    pub selection: Option<SelectionShape>,
    pub text_hitbox: Option<Hitbox>,
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
            self.debug_interior_corners,
        );

        let text_hitbox = if line.width > Pixels::ZERO {
            let text_bounds = Bounds {
                origin: bounds.origin,
                size: size(line.width, bounds.size.height),
            };
            Some(window.insert_hitbox(text_bounds, HitboxBehavior::Normal))
        } else {
            None
        };

        LinePrepaintState {
            line: Some(line),
            selection,
            text_hitbox,
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

        if let Some(hitbox) = &prepaint.text_hitbox {
            window.set_cursor_style(CursorStyle::IBeam, hitbox);
        }

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
    pub debug_interior_corners: bool,
}

pub(crate) struct WrappedLinePrepaintState {
    pub line: Option<ShapedLine>,
    pub selection: Option<SelectionShape>,
    pub text_hitbox: Option<Hitbox>,
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
                text_hitbox: None,
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
            self.debug_interior_corners,
        );

        let text_hitbox = if line.width > Pixels::ZERO {
            let text_bounds = Bounds {
                origin: bounds.origin,
                size: size(line.width, bounds.size.height),
            };
            Some(window.insert_hitbox(text_bounds, HitboxBehavior::Normal))
        } else {
            None
        };

        WrappedLinePrepaintState {
            line: Some(line),
            selection,
            text_hitbox,
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

        if let Some(hitbox) = &prepaint.text_hitbox {
            window.set_cursor_style(CursorStyle::IBeam, hitbox);
        }

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

/// Custom element that uses `request_measured_layout` with the user's actual style
/// (width, max-width, etc.) to be the Taffy leaf node directly. This eliminates the
/// parent-child height mismatch: since this element IS the container, Taffy's
/// measurement directly determines the container's height.
///
/// The measure callback wraps text at the actual available width and returns the
/// correct height, which Taffy uses for this node's bounds. No parent div needed.
///
/// Children (WrappedLineElements) are created in prepaint and painted in paint,
/// similar to how GPUI's uniform_list manages its items.
pub(crate) struct WrappedTextElement {
    pub state: Entity<SelectableTextState>,
    pub text_color: Hsla,
    pub highlight_text_color: Hsla,
    pub line_height: Pixels,
    pub font_size: Pixels,
    pub font: Font,
    pub selected_range: Range<usize>,
    pub selection_rounded: Option<Pixels>,
    pub selection_rounded_smoothing: Option<f32>,
    pub debug_character_bounds: bool,
    pub debug_interior_corners: bool,
    pub debug_wrapping: bool,
    pub line_clamp: usize,
    pub scale_factor: f32,
    pub style: Style,
    /// Created during prepaint, painted during paint.
    pub children: Vec<WrappedLineElement>,
}

pub(crate) struct WrappedTextPrepaintState {
    child_prepaints: Vec<WrappedLinePrepaintState>,
}

impl WrappedTextElement {
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
}

impl IntoElement for WrappedTextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for WrappedTextElement {
    type RequestLayoutState = ();
    type PrepaintState = WrappedTextPrepaintState;

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
        let state = self.state.clone();
        let line_height = self.line_height;
        let line_clamp = self.line_clamp;
        let scale_factor = self.scale_factor;
        let font = self.font.clone();
        let font_size = self.font_size;
        let text_color = self.text_color;

        // Pass the user's actual style (w, max_w, etc.) to request_measured_layout.
        // This makes THIS element the Taffy leaf node with the user's constraints,
        // so Taffy resolves the correct available width and our measure callback
        // returns the correct height directly on this node - no parent needed.
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
                    let visible = line_clamp.min(count).max(1);
                    let height = multiline_height(line_height, visible, scale_factor);
                    return size(Pixels::ZERO, height);
                };

                let wrap_width = width + WIDTH_WRAP_BASE_MARGIN;
                let text = state.read(cx).get_text();

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
                    state.measured_max_line_width = Some(window.round(max_line_width));
                    state.precomputed_at_width = Some(wrap_width);
                    state.precomputed_visual_lines = visual_lines;
                    state.precomputed_wrapped_lines = wrapped_lines;
                    state.cached_wrap_width = Some(width);

                    if state.scroll_to_cursor_on_next_render {
                        state.scroll_to_cursor_on_next_render = false;
                        state.ensure_cursor_visible();
                    }
                });

                let visible_lines = line_clamp.min(visual_line_count).max(1);
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
    ) -> Self::PrepaintState {
        // Wrapping was already computed in the measure callback with the correct width.
        // Create children with the exact count needed.
        let actual_line_count = self.state.read(cx).precomputed_visual_lines.len().max(1);
        let visual_lines = self.state.read(cx).precomputed_visual_lines.clone();

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
                state: self.state.clone(),
                visual_line_index: visual_idx,
                text_color: self.text_color,
                highlight_text_color: self.highlight_text_color,
                line_height: self.line_height,
                font_size: self.font_size,
                font: self.font.clone(),
                selected_range: self.selected_range.clone(),
                selection_rounded: self.selection_rounded,
                selection_rounded_smoothing: self.selection_rounded_smoothing,
                prev_visual_line_offsets,
                next_visual_line_offsets,
                debug_character_bounds: self.debug_character_bounds,
                debug_interior_corners: self.debug_interior_corners,
            });
        }

        // Prepaint children, positioning them manually at line_height intervals.
        let mut child_prepaints = Vec::with_capacity(actual_line_count);
        for (idx, child) in self.children.iter_mut().enumerate() {
            let child_bounds = Bounds {
                origin: point(
                    bounds.origin.x,
                    bounds.origin.y + self.line_height * idx as f32,
                ),
                size: gpui::Size {
                    width: bounds.size.width,
                    height: self.line_height,
                },
            };

            let prepaint_state = child.prepaint(None, None, child_bounds, &mut (), window, cx);
            child_prepaints.push(prepaint_state);
        }

        WrappedTextPrepaintState { child_prepaints }
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
        let state = self.state.clone();
        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
            if phase == DispatchPhase::Capture {
                return;
            }

            state.update(cx, |state, cx| {
                if state.is_selecting {
                    state.pending_selection_position = Some(event.position);
                    cx.notify();
                }
            });
        });

        self.state.update(cx, |state, _cx| {
            state.visible_lines_info.clear();
        });

        // Paint children with content mask for clipping
        let visible_lines = self.line_clamp.min(self.children.len()).max(1);
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
                            bounds.origin.y + self.line_height * idx as f32,
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
    /// Always calls cx.notify() when the width changes to ensure a re-render is scheduled,
    /// even if GPUI skips render() during fast resize sequences.
    fn check_container_width_change(&self, actual_width: Pixels, cx: &mut App) {
        if actual_width <= Pixels::ZERO {
            return;
        }

        let (cached_wrap_width, needs_wrap_recompute) = {
            let state = self.state.read(cx);
            (state.cached_wrap_width, state.needs_wrap_recompute)
        };

        let cached = cached_wrap_width.unwrap_or(Pixels::ZERO);
        let changed = (actual_width - cached).abs() > px(0.01);

        if changed || cached_wrap_width.is_none() {
            self.state.update(cx, |state, cx| {
                state.cached_wrap_width = Some(actual_width);
                if !needs_wrap_recompute {
                    state.needs_wrap_recompute = true;
                }
                cx.notify();
            });
        }
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
        self.check_container_width_change(bounds.size.width, cx);

        // If the container has SHRUNK since last render, re-wrap text at the actual
        // narrower width BEFORE child elements prepaint. This prevents partially-clipped
        // words (e.g. "don'" instead of "don't") during fast resize.
        // Only re-wrap on shrink - on grow, the old narrower wrap is safe (text just
        // doesn't fill the width for 1 frame). Re-wrapping on grow would produce fewer
        // visual lines than uniform_list slots, causing text to vanish for 1 frame.
        let actual_width = bounds.size.width;
        if actual_width > Pixels::ZERO {
            let precomputed_at = self.state.read(cx).precomputed_at_width;
            let expected_wrap_width = actual_width + WIDTH_WRAP_BASE_MARGIN;
            if let Some(prev_width) = precomputed_at {
                if expected_wrap_width < prev_width - px(0.01) {
                    self.state.update(cx, |state, _cx| {
                        state.rewrap_at_width(actual_width, window);
                    });
                }
            }
        }

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
