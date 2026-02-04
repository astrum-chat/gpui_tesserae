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

/// Computes which corners should be rounded based on adjacent line selection positions.
///
/// For multi-line selections, corners are rounded when they are at the outer edge
/// of the selection shape. Inner corners (where the selection wraps to a new line)
/// remain sharp to create a cohesive selection appearance.
///
/// - `this_start_x`: X-coordinate where selection starts on this line
/// - `this_end_x`: X-coordinate where selection ends on this line
/// - `prev_line`: Selection bounds (start_x, end_x) of the previous line, if any
/// - `next_line`: Selection bounds (start_x, end_x) of the next line, if any
/// - `radius`: The corner radius to apply
pub fn compute_selection_corners(
    this_start_x: Pixels,
    this_end_x: Pixels,
    prev_line: Option<(Pixels, Pixels)>,
    next_line: Option<(Pixels, Pixels)>,
    radius: Pixels,
) -> Corners<Pixels> {
    let zero = Pixels::ZERO;

    // Top-left: rounded if first line OR this line starts at/before previous line's start
    let top_left = match prev_line {
        None => radius,
        Some((prev_start, _)) => {
            if this_start_x <= prev_start {
                radius
            } else {
                zero
            }
        }
    };

    // Top-right: rounded if first line OR this line ends at/after previous line's end
    let top_right = match prev_line {
        None => radius,
        Some((_, prev_end)) => {
            if this_end_x >= prev_end {
                radius
            } else {
                zero
            }
        }
    };

    // Bottom-left: rounded if last line OR this line starts at/before next line's start
    let bottom_left = match next_line {
        None => radius,
        Some((next_start, _)) => {
            if this_start_x <= next_start {
                radius
            } else {
                zero
            }
        }
    };

    // Bottom-right: rounded if last line OR this line ends at/after next line's end
    let bottom_right = match next_line {
        None => radius,
        Some((_, next_end)) => {
            if this_end_x >= next_end {
                radius
            } else {
                zero
            }
        }
    };

    Corners {
        top_left,
        top_right,
        bottom_right,
        bottom_left,
    }
}

/// Determines if trailing whitespace should be shown in selection highlighting.
/// This ensures that when a selection spans multiple lines, the newline character
/// is visually represented by extending the selection highlight.
///
/// Note: This function handles both logical lines (ending with newline) and visual
/// lines from word wrapping (which may not end with newline). Trailing whitespace
/// should only be shown when there's an actual newline at the end of the line.
pub fn should_show_trailing_whitespace(
    selected_range: &Range<usize>,
    line_end_offset: usize,
    is_select_all: bool,
) -> bool {
    if !is_select_all && selected_range.end == line_end_offset {
        false
    } else {
        true
    }
}
