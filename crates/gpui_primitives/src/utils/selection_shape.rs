//! Selection shape rendering utilities.
//!
//! This module provides types and functions for rendering text selection highlights,
//! supporting both simple rectangular selections and smooth squircle-style corners.

use figma_squircle::{FigmaSquircleParams, get_svg_path};
use gpui::{
    BorderStyle, Bounds, Corners, Edges, Hsla, PaintQuad, Path, PathBuilder, Pixels, Window, fill,
    point, px,
};
use lyon::extra::parser::{ParserOptions, PathParser, Source};
use lyon::path::Path as LyonPath;

/// Configuration for selection shape rendering.
#[derive(Clone, Copy, Debug, Default)]
pub struct SelectionShapeConfig {
    /// Corner radius for rounded corners (0 = sharp).
    pub corner_radius: Pixels,
    /// Corner smoothing factor (0.0 = standard rounded rect, 1.0 = full squircle).
    /// When Some and > 0, uses squircle rendering instead of PaintQuad.
    pub corner_smoothing: Option<f32>,
}

impl SelectionShapeConfig {
    /// Creates a simple rectangular selection (no rounding).
    pub fn rectangle() -> Self {
        Self::default()
    }

    /// Creates a rounded rectangle selection (fast path).
    pub fn rounded(radius: Pixels) -> Self {
        Self {
            corner_radius: radius,
            corner_smoothing: None,
        }
    }

    /// Creates a squircle selection with smoothing.
    pub fn squircle(radius: Pixels, smoothing: f32) -> Self {
        Self {
            corner_radius: radius,
            corner_smoothing: Some(smoothing.clamp(0.0, 1.0)),
        }
    }

    /// Returns true if this config uses squircle rendering.
    pub fn uses_squircle(&self) -> bool {
        self.corner_smoothing.map_or(false, |s| s > 0.0) && self.corner_radius > Pixels::ZERO
    }
}

/// A paintable selection shape, either a simple quad or a squircle path.
pub enum SelectionShape {
    /// Simple rectangle or rounded rectangle (uses PaintQuad).
    Quad(PaintQuad),
    /// Squircle path for smooth corners.
    Squircle {
        path: Path<Pixels>,
        fill_color: Hsla,
    },
}

impl SelectionShape {
    /// Paints the selection shape to the window.
    pub fn paint(self, window: &mut Window) {
        match self {
            SelectionShape::Quad(quad) => window.paint_quad(quad),
            SelectionShape::Squircle { path, fill_color } => {
                window.paint_path(path, fill_color);
            }
        }
    }
}

/// Returns whether a corner at `x` is covered by an adjacent line's selection range.
fn is_corner_covered(x: Pixels, adjacent_line: Option<(Pixels, Pixels)>) -> bool {
    adjacent_line.map_or(false, |(start, end)| x >= start && x <= end)
}

/// Computes which corners of a selection rectangle should be rounded.
///
/// For multi-line selections, a corner is rounded when "exposed" (not covered by
/// the adjacent line's selection). This creates a cohesive shape where inner
/// corners remain sharp and outer edges are rounded.
pub fn compute_selection_corners(
    this_start_x: Pixels,
    this_end_x: Pixels,
    prev_line: Option<(Pixels, Pixels)>,
    next_line: Option<(Pixels, Pixels)>,
    radius: Pixels,
) -> Corners<Pixels> {
    let round_if_exposed = |x: Pixels, adjacent: Option<(Pixels, Pixels)>| -> Pixels {
        if is_corner_covered(x, adjacent) {
            Pixels::ZERO
        } else {
            radius
        }
    };

    Corners {
        top_left: round_if_exposed(this_start_x, prev_line),
        top_right: round_if_exposed(this_end_x, prev_line),
        bottom_left: round_if_exposed(this_start_x, next_line),
        bottom_right: round_if_exposed(this_end_x, next_line),
    }
}

/// Creates a selection quad (highlighted background) for text selection rendering.
pub fn make_selection_quad(
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

/// Creates a selection quad with custom corner radii for rounded selection rendering.
pub fn make_selection_quad_rounded(
    bounds: Bounds<Pixels>,
    start_x: Pixels,
    end_x: Pixels,
    scroll_offset: Pixels,
    highlight_color: Hsla,
    corner_radii: Corners<Pixels>,
) -> PaintQuad {
    PaintQuad {
        bounds: Bounds::from_corners(
            point(bounds.left() + start_x - scroll_offset, bounds.top()),
            point(bounds.left() + end_x - scroll_offset, bounds.bottom()),
        ),
        corner_radii,
        background: highlight_color.into(),
        border_widths: Edges::default(),
        border_color: Hsla::transparent_black(),
        border_style: BorderStyle::default(),
    }
}

/// Creates a SelectionShapeConfig from optional radius and smoothing values.
pub fn selection_config_from_options(
    radius: Option<Pixels>,
    smoothing: Option<f32>,
) -> SelectionShapeConfig {
    match (radius, smoothing) {
        (Some(r), Some(s)) if r > Pixels::ZERO && s > 0.0 => SelectionShapeConfig::squircle(r, s),
        (Some(r), _) if r > Pixels::ZERO => SelectionShapeConfig::rounded(r),
        _ => SelectionShapeConfig::rectangle(),
    }
}

/// Builds a selection shape for the given bounds and configuration.
///
/// Chooses between the fast PaintQuad path (no smoothing) and squircle path (smoothing > 0).
pub fn build_selection_shape(
    bounds: Bounds<Pixels>,
    start_x: Pixels,
    end_x: Pixels,
    scroll_offset: Pixels,
    highlight_color: Hsla,
    config: &SelectionShapeConfig,
    corners: Corners<Pixels>,
) -> SelectionShape {
    if !config.uses_squircle() {
        let has_rounded_corners = corners.top_left > Pixels::ZERO
            || corners.top_right > Pixels::ZERO
            || corners.bottom_left > Pixels::ZERO
            || corners.bottom_right > Pixels::ZERO;

        if has_rounded_corners {
            return SelectionShape::Quad(make_selection_quad_rounded(
                bounds,
                start_x,
                end_x,
                scroll_offset,
                highlight_color,
                corners,
            ));
        } else {
            return SelectionShape::Quad(make_selection_quad(
                bounds,
                start_x,
                end_x,
                scroll_offset,
                highlight_color,
            ));
        }
    }

    let smoothing = config.corner_smoothing.unwrap_or(0.0);
    let selection_bounds = Bounds::from_corners(
        point(bounds.left() + start_x - scroll_offset, bounds.top()),
        point(bounds.left() + end_x - scroll_offset, bounds.bottom()),
    );

    match build_squircle_path(selection_bounds, corners, smoothing, highlight_color) {
        Some(shape) => shape,
        None => SelectionShape::Quad(make_selection_quad_rounded(
            bounds,
            start_x,
            end_x,
            scroll_offset,
            highlight_color,
            corners,
        )),
    }
}

fn build_squircle_path(
    bounds: Bounds<Pixels>,
    corners: Corners<Pixels>,
    smoothing: f32,
    fill_color: Hsla,
) -> Option<SelectionShape> {
    let width = bounds.size.width.to_f64() as f32;
    let height = bounds.size.height.to_f64() as f32;

    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    let svg_path = get_svg_path(
        FigmaSquircleParams::default()
            .width(width)
            .height(height)
            .top_left_corner_radius(corners.top_left.to_f64() as f32)
            .top_right_corner_radius(corners.top_right.to_f64() as f32)
            .bottom_right_corner_radius(corners.bottom_right.to_f64() as f32)
            .bottom_left_corner_radius(corners.bottom_left.to_f64() as f32)
            .corner_smoothing(smoothing)
            .preserve_smoothing(true),
    );

    let lyon_path = parse_svg_path(&svg_path)?;
    let gpui_path = lyon_to_gpui(lyon_path, bounds.origin.x, bounds.origin.y)?;

    Some(SelectionShape::Squircle {
        path: gpui_path,
        fill_color,
    })
}

fn parse_svg_path(svg_path: &str) -> Option<LyonPath> {
    let mut builder = LyonPath::builder();
    PathParser::new()
        .parse(
            &ParserOptions::DEFAULT,
            &mut Source::new(svg_path.chars()),
            &mut builder,
        )
        .ok()?;
    Some(builder.build())
}

fn lyon_to_gpui(lyon_path: LyonPath, origin_x: Pixels, origin_y: Pixels) -> Option<Path<Pixels>> {
    let mut builder = PathBuilder::fill();

    for event in lyon_path.iter() {
        match event {
            lyon::path::Event::Begin { at } => {
                let at = point(origin_x + px(at.x), origin_y + px(at.y));
                builder.move_to(at);
            }
            lyon::path::Event::Line { from: _, to } => {
                let to = point(origin_x + px(to.x), origin_y + px(to.y));
                builder.line_to(to);
            }
            lyon::path::Event::Quadratic { from: _, ctrl, to } => {
                let ctrl = point(origin_x + px(ctrl.x), origin_y + px(ctrl.y));
                let to = point(origin_x + px(to.x), origin_y + px(to.y));
                builder.curve_to(to, ctrl);
            }
            lyon::path::Event::Cubic {
                from: _,
                ctrl1,
                ctrl2,
                to,
            } => {
                let ctrl1 = point(origin_x + px(ctrl1.x), origin_y + px(ctrl1.y));
                let ctrl2 = point(origin_x + px(ctrl2.x), origin_y + px(ctrl2.y));
                let to = point(origin_x + px(to.x), origin_y + px(to.y));
                builder.cubic_bezier_to(to, ctrl1, ctrl2);
            }
            lyon::path::Event::End { close, .. } => {
                if close {
                    builder.close();
                }
            }
        }
    }

    builder.build().ok()
}
