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

/// A single paintable primitive: either a quad or a path.
pub(crate) enum SelectionPrimitive {
    /// Simple rectangle or rounded rectangle (uses PaintQuad).
    Quad(PaintQuad),
    /// Path-based shape for smooth corners or concave patches.
    Path {
        path: Path<Pixels>,
        fill_color: Hsla,
    },
}

impl SelectionPrimitive {
    fn paint(self, window: &mut Window) {
        match self {
            SelectionPrimitive::Quad(quad) => window.paint_quad(quad),
            SelectionPrimitive::Path { path, fill_color } => {
                window.paint_path(path, fill_color);
            }
        }
    }
}

/// A paintable selection shape: the main selection rectangle plus any interior corner patches.
pub struct SelectionShape {
    /// The main selection rectangle (possibly with rounded exterior corners).
    shape: SelectionPrimitive,
    /// Small concave corner patches painted at interior (step) corners.
    interior_corners: Vec<SelectionPrimitive>,
}

impl SelectionShape {
    /// Creates a SelectionShape from a primitive and interior corner patches.
    pub(crate) fn new(
        shape: SelectionPrimitive,
        interior_corners: Vec<SelectionPrimitive>,
    ) -> Self {
        Self {
            shape,
            interior_corners,
        }
    }

    /// Paints the selection shape and all interior corner patches to the window.
    pub fn paint(self, window: &mut Window) {
        self.shape.paint(window);
        for corner in self.interior_corners {
            corner.paint(window);
        }
    }
}

/// Returns whether a left (start) corner is covered by an adjacent line's selection.
/// Covered when: edges are aligned (within subpixel tolerance), or the adjacent line
/// extends past this edge by at least `radius` (so its rounding doesn't create a gap).
fn is_left_corner_covered(
    this_start: Pixels,
    adjacent_line: Option<(Pixels, Pixels)>,
    subpixel_tolerance: Pixels,
    radius: Pixels,
) -> bool {
    adjacent_line.map_or(false, |(adj_start, adj_end)| {
        // Adjacent selection must reach this corner (overlap check)
        if adj_end < this_start - subpixel_tolerance {
            return false;
        }
        let diff = this_start - adj_start;
        diff.abs() <= subpixel_tolerance || diff >= radius
    })
}

/// Returns whether a right (end) corner is covered by an adjacent line's selection.
/// Covered when: edges are aligned (within subpixel tolerance), or the adjacent line
/// extends past this edge by at least `radius` (so its rounding doesn't create a gap).
fn is_right_corner_covered(
    this_end: Pixels,
    adjacent_line: Option<(Pixels, Pixels)>,
    subpixel_tolerance: Pixels,
    radius: Pixels,
) -> bool {
    adjacent_line.map_or(false, |(adj_start, adj_end)| {
        // Adjacent selection must reach this corner (overlap check)
        if adj_start > this_end + subpixel_tolerance {
            return false;
        }
        let diff = adj_end - this_end;
        diff.abs() <= subpixel_tolerance || diff >= radius
    })
}

/// Computes which corners of a selection rectangle should be rounded.
///
/// A corner is sharp (not rounded) when:
/// - The adjacent line's edge aligns with this line's edge (within subpixel tolerance), OR
/// - The adjacent line extends past this line by at least `radius` (fully covering the corner).
/// Otherwise the corner is rounded with the full radius.
pub fn compute_selection_corners(
    this_start_x: Pixels,
    this_end_x: Pixels,
    prev_line: Option<(Pixels, Pixels)>,
    next_line: Option<(Pixels, Pixels)>,
    radius: Pixels,
    scale_factor: f32,
) -> Corners<Pixels> {
    let sp = px(scale_factor / 2.0);
    let round = |covered: bool| -> Pixels { if covered { Pixels::ZERO } else { radius } };

    Corners {
        top_left: round(is_left_corner_covered(this_start_x, prev_line, sp, radius)),
        top_right: round(is_right_corner_covered(this_end_x, prev_line, sp, radius)),
        bottom_left: round(is_left_corner_covered(this_start_x, next_line, sp, radius)),
        bottom_right: round(is_right_corner_covered(this_end_x, next_line, sp, radius)),
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

/// Builds a selection shape with no interior corners. Used for single-line selections.
pub fn build_selection_shape(
    bounds: Bounds<Pixels>,
    start_x: Pixels,
    end_x: Pixels,
    scroll_offset: Pixels,
    highlight_color: Hsla,
    config: &SelectionShapeConfig,
    corners: Corners<Pixels>,
) -> SelectionShape {
    SelectionShape {
        shape: build_selection_primitive(
            bounds,
            start_x,
            end_x,
            scroll_offset,
            highlight_color,
            config,
            corners,
        ),
        interior_corners: Vec::new(),
    }
}

/// Builds a selection primitive for the given bounds and configuration.
///
/// Chooses between the fast PaintQuad path (no smoothing) and squircle path (smoothing > 0).
pub(crate) fn build_selection_primitive(
    bounds: Bounds<Pixels>,
    start_x: Pixels,
    end_x: Pixels,
    scroll_offset: Pixels,
    highlight_color: Hsla,
    config: &SelectionShapeConfig,
    corners: Corners<Pixels>,
) -> SelectionPrimitive {
    if !config.uses_squircle() {
        let has_rounded_corners = corners.top_left > Pixels::ZERO
            || corners.top_right > Pixels::ZERO
            || corners.bottom_left > Pixels::ZERO
            || corners.bottom_right > Pixels::ZERO;

        if has_rounded_corners {
            return SelectionPrimitive::Quad(make_selection_quad_rounded(
                bounds,
                start_x,
                end_x,
                scroll_offset,
                highlight_color,
                corners,
            ));
        } else {
            return SelectionPrimitive::Quad(make_selection_quad(
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
        Some(primitive) => primitive,
        None => SelectionPrimitive::Quad(make_selection_quad_rounded(
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
) -> Option<SelectionPrimitive> {
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

    Some(SelectionPrimitive::Path {
        path: gpui_path,
        fill_color,
    })
}

/// Orientation of a concave (interior) corner at a step between two adjacent selection lines.
#[derive(Clone, Copy, Debug)]
enum ConcaveCornerPosition {
    /// Upper line ends earlier on the right; patch fills top-right of the step.
    TopRight,
    /// Upper line starts later on the left; patch fills top-left of the step.
    TopLeft,
    /// Lower line ends earlier on the right; patch fills bottom-right of the step.
    BottomRight,
    /// Lower line starts later on the left; patch fills bottom-left of the step.
    BottomLeft,
}

/// Cubic bezier approximation constant for a quarter circle arc.
const KAPPA: f32 = 0.5522847498;

/// Builds a concave (interior) corner patch as a filled path.
///
/// `rx` is the horizontal extent, `ry` is the vertical extent. When the step between
/// adjacent lines is smaller than the full patch size, `rx` is clamped to the step so the
/// patch doesn't extend past the adjacent line's selection edge.
///
/// When `corner_smoothing` is `Some` and > 0, uses the squircle algorithm for a smoother
/// curve that matches the exterior corner style. Otherwise uses a simple circular bezier
/// approximation (faster hot path).
fn build_concave_corner_path(
    cx: Pixels,
    cy: Pixels,
    rx: Pixels,
    ry: Pixels,
    position: ConcaveCornerPosition,
    fill_color: Hsla,
    corner_smoothing: Option<f32>,
) -> Option<SelectionPrimitive> {
    if rx <= Pixels::ZERO || ry <= Pixels::ZERO {
        return None;
    }

    let smoothing = corner_smoothing.unwrap_or(0.0);

    // When smoothing > 0, use the concave squircle path for consistency with exterior corners.
    if smoothing > 0.0 {
        use crate::utils::concave_squircle::{self, ConcaveCorner};
        let corner = match position {
            ConcaveCornerPosition::TopRight => ConcaveCorner::TopRight,
            ConcaveCornerPosition::TopLeft => ConcaveCorner::TopLeft,
            ConcaveCornerPosition::BottomRight => ConcaveCorner::BottomRight,
            ConcaveCornerPosition::BottomLeft => ConcaveCorner::BottomLeft,
        };
        let path =
            concave_squircle::build_concave_squircle_path(cx, cy, rx, ry, corner, smoothing)?;
        return Some(SelectionPrimitive::Path { path, fill_color });
    }

    // Hot path: simple circular bezier approximation (smoothing == 0).
    let kx = rx * KAPPA;
    let ky = ry * KAPPA;
    let mut builder = PathBuilder::fill();

    match position {
        ConcaveCornerPosition::TopRight => {
            builder.move_to(point(cx, cy));
            builder.line_to(point(cx + rx, cy));
            builder.cubic_bezier_to(
                point(cx, cy - ry),
                point(cx + rx - kx, cy),
                point(cx, cy - ry + ky),
            );
            builder.close();
        }
        ConcaveCornerPosition::TopLeft => {
            builder.move_to(point(cx, cy));
            builder.line_to(point(cx - rx, cy));
            builder.cubic_bezier_to(
                point(cx, cy - ry),
                point(cx - rx + kx, cy),
                point(cx, cy - ry + ky),
            );
            builder.close();
        }
        ConcaveCornerPosition::BottomRight => {
            builder.move_to(point(cx, cy));
            builder.line_to(point(cx + rx, cy));
            builder.cubic_bezier_to(
                point(cx, cy + ry),
                point(cx + rx - kx, cy),
                point(cx, cy + ry - ky),
            );
            builder.close();
        }
        ConcaveCornerPosition::BottomLeft => {
            builder.move_to(point(cx, cy));
            builder.line_to(point(cx - rx, cy));
            builder.cubic_bezier_to(
                point(cx, cy + ry),
                point(cx - rx + kx, cy),
                point(cx, cy + ry - ky),
            );
            builder.close();
        }
    }

    let path = builder.build().ok()?;
    Some(SelectionPrimitive::Path { path, fill_color })
}

/// Minimum step size (in pixels) below which interior corner patches are skipped.
/// Steps smaller than this don't have enough room for a visible concave curve.
const MIN_INTERIOR_CORNER_STEP: Pixels = px(2.0);

/// Computes interior (concave) corner patches for a selection line.
///
/// Interior corners exist at "steps" where adjacent selection lines have different widths.
/// Each patch is a small filled shape that rounds the concave corner at the step.
///
/// `bounds_left` is the absolute left edge of the line's render bounds.
/// `line_height` is the vertical extent of the line — used as the patch size in both
/// x and y directions so concave patches visually match the convex outer corners.
/// `scroll_offset` is subtracted from x-coordinates for scrolled views.
pub(crate) fn compute_interior_corner_patches(
    this_start_x: Pixels,
    this_end_x: Pixels,
    prev_line: Option<(Pixels, Pixels)>,
    next_line: Option<(Pixels, Pixels)>,
    radius: Pixels,
    corner_smoothing: Option<f32>,
    scale_factor: f32,
    bounds_left: Pixels,
    bounds_top: Pixels,
    bounds_bottom: Pixels,
    line_height: Pixels,
    scroll_offset: Pixels,
    fill_color: Hsla,
) -> Vec<SelectionPrimitive> {
    if radius <= Pixels::ZERO {
        return Vec::new();
    }

    // Patch size = diameter (radius * 2), clamped to line_height - radius.
    let patch_size = (radius * 2.0).min(line_height - radius).max(Pixels::ZERO);
    if patch_size <= Pixels::ZERO {
        return Vec::new();
    }

    let sp = px(scale_factor / 2.0);
    let mut patches = Vec::new();

    // Helper: builds an interior corner patch and pushes it if the step is large enough.
    // Skips when the step is smaller than the corner radius — in that range the adjacent
    // line's rounded exterior corner already covers the visual transition.
    fn try_push_patch(
        patches: &mut Vec<SelectionPrimitive>,
        step: Pixels,
        radius: Pixels,
        patch_size: Pixels,
        cx: Pixels,
        cy: Pixels,
        position: ConcaveCornerPosition,
        fill_color: Hsla,
        corner_smoothing: Option<f32>,
    ) {
        if step < MIN_INTERIOR_CORNER_STEP {
            return;
        }
        // The adjacent line's rounded exterior corner occupies space at the step
        // position. Subtract half the radius so the interior patch doesn't overlap.
        let available = step - radius / 2.0;
        if available < MIN_INTERIOR_CORNER_STEP {
            return;
        }
        let rx = patch_size.min(available);
        // Skip if the clamped width is less than half the full patch size —
        // too narrow to render a recognizable concave curve.
        if rx < patch_size * 0.25 {
            return;
        }
        let Some(patch) = build_concave_corner_path(
            cx,
            cy,
            rx,
            patch_size,
            position,
            fill_color,
            corner_smoothing,
        ) else {
            return;
        };
        patches.push(patch);
    }

    // Check top edge (relationship with prev_line)
    if let Some((prev_start, prev_end)) = prev_line {
        // Top-right interior: this line extends further right than prev line.
        // Skip if no horizontal overlap.
        if this_end_x > prev_end + sp && prev_end > this_start_x - sp {
            let step = this_end_x - prev_end;
            let cx = bounds_left + prev_end - scroll_offset;
            try_push_patch(
                &mut patches,
                step,
                radius,
                patch_size,
                cx,
                bounds_top,
                ConcaveCornerPosition::TopRight,
                fill_color,
                corner_smoothing,
            );
        }

        // Top-left interior: this line extends further left than prev line.
        if this_start_x < prev_start - sp && prev_start < this_end_x + sp {
            let step = prev_start - this_start_x;
            let cx = bounds_left + prev_start - scroll_offset;
            try_push_patch(
                &mut patches,
                step,
                radius,
                patch_size,
                cx,
                bounds_top,
                ConcaveCornerPosition::TopLeft,
                fill_color,
                corner_smoothing,
            );
        }
    }

    // Check bottom edge (relationship with next_line)
    if let Some((next_start, next_end)) = next_line {
        // Bottom-right interior: this line extends further right than next line.
        if this_end_x > next_end + sp && next_end > this_start_x - sp {
            let step = this_end_x - next_end;
            let cx = bounds_left + next_end - scroll_offset;
            try_push_patch(
                &mut patches,
                step,
                radius,
                patch_size,
                cx,
                bounds_bottom,
                ConcaveCornerPosition::BottomRight,
                fill_color,
                corner_smoothing,
            );
        }

        // Bottom-left interior: this line extends further left than next line.
        if this_start_x < next_start - sp && next_start < this_end_x + sp {
            let step = next_start - this_start_x;
            let cx = bounds_left + next_start - scroll_offset;
            try_push_patch(
                &mut patches,
                step,
                radius,
                patch_size,
                cx,
                bounds_bottom,
                ConcaveCornerPosition::BottomLeft,
                fill_color,
                corner_smoothing,
            );
        }
    }

    patches
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
