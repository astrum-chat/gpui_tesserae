use std::ops::Range;

use gpui::{BorderStyle, Bounds, Corners, Edges, Hsla, PaintQuad, Pixels, fill, point, px, size};

/// Small epsilon used when comparing wrap widths to prevent janky text wrapping
/// caused by floating point precision issues triggering unnecessary recomputes.
pub const WRAP_WIDTH_EPSILON: Pixels = px(1.25);

/// Rounds a pixel value to the nearest pixel-perfect increment based on scale factor.
pub fn pixel_perfect_round(value: Pixels, scale_factor: f32) -> Pixels {
    let increment = if scale_factor >= 2.0 { 0.5 } else { 1.0 };
    let val = value.to_f64() as f32;
    px((val / increment).round() * increment)
}

/// Calculates the height for a multiline text area based on line height and count.
pub fn multiline_height(line_height: Pixels, line_count: usize, scale_factor: f32) -> Pixels {
    let height = line_height * line_count as f32;
    pixel_perfect_round(height, scale_factor)
}

/// Creates a cursor quad (thin vertical line) for text cursor rendering.
pub fn make_cursor_quad(
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

/// Determines if trailing whitespace should be shown in selection highlighting.
///
/// When a selection spans multiple lines, extending the highlight past the line's
/// text visually represents the newline character. This is suppressed when the
/// selection ends exactly at the line end (unless select-all is active).
pub fn should_show_trailing_whitespace(
    selected_range: &Range<usize>,
    line_end_offset: usize,
    is_select_all: bool,
) -> bool {
    is_select_all || selected_range.end != line_end_offset
}
