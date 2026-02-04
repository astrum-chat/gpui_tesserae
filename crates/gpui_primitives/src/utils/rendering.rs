use std::ops::Range;

use gpui::{Bounds, Hsla, PaintQuad, Pixels, fill, point, px, size};

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
