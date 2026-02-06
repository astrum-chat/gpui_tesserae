use std::ops::Range;

use gpui::{Bounds, Hsla, PaintQuad, Pixels, fill, point, px, size};

/// Base margin added to wrap widths to prevent janky text wrapping.
/// The full margin used when computing widths is `whitespace_width + WIDTH_WRAP_BASE_MARGIN`.
/// Also used as a threshold when comparing wrap widths for change detection.
pub const WIDTH_WRAP_BASE_MARGIN: Pixels = px(1.25);

/// Calculates the height for a multiline text area based on line height and count.
pub fn multiline_height(line_height: Pixels, line_count: usize, scale_factor: f32) -> Pixels {
    let height = line_height * line_count as f32;
    let increment = if scale_factor >= 2.0 { 0.5 } else { 1.0 };
    let val = height.to_f64() as f32;
    px((val / increment).round() * increment)
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
pub fn should_show_trailing_whitespace(
    selected_range: &Range<usize>,
    line_end_offset: usize,
) -> bool {
    selected_range.end > line_end_offset
}
