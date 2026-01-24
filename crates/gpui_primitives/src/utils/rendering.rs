use std::ops::Range;

use gpui::{Bounds, Hsla, PaintQuad, Pixels, fill, point, px, size};

/// Small epsilon used when comparing wrap widths to prevent janky text wrapping
/// caused by floating point precision issues triggering unnecessary recomputes.
pub const WRAP_WIDTH_EPSILON: Pixels = px(1.5);

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
