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
    line_start_offset: usize,
    line_end_offset: usize,
    line_len: usize,
    local_end: usize,
    text: &str,
) -> bool {
    // Check if this line actually ends with a newline (or is at the end of text).
    // For wrapped visual lines that don't end at a newline, we shouldn't show
    // trailing whitespace since there's no newline to represent.
    let line_ends_with_newline = text
        .get(line_end_offset..line_end_offset + 1)
        .map(|c| c == "\n")
        .unwrap_or(true); // End of text is treated like a newline

    if !line_ends_with_newline {
        return false;
    }

    let newline_position = line_end_offset;

    let selection_starts_at_newline = text
        .get(selected_range.start..selected_range.start + 1)
        .map(|c| c == "\n")
        .unwrap_or(false);

    let selection_continues_past_newline = selected_range.end > newline_position;
    let at_line_end = local_end == line_len;

    let selection_starts_at_line_start = selected_range.start == line_start_offset;

    // Only skip trailing whitespace for the starting newline if we're on the line where
    // the selection actually starts. This prevents disabling trailing whitespace for
    // the entire selection when it starts at a newline character.
    let on_selection_start_line =
        selected_range.start >= line_start_offset && selected_range.start <= line_end_offset;
    let skip_for_starting_newline = selection_starts_at_newline && on_selection_start_line;

    let selection_includes_current_line = selected_range.start <= line_end_offset;

    let standard_trailing = !skip_for_starting_newline
        && selection_continues_past_newline
        && at_line_end
        && selection_includes_current_line;
    let starts_at_line_start =
        selection_starts_at_line_start && at_line_end && selected_range.end > line_end_offset;

    // For empty lines (just a newline) that are entirely within the selection
    let empty_line_in_selection = line_len == 0
        && selected_range.start <= line_start_offset
        && selected_range.end > line_end_offset;

    standard_trailing || starts_at_line_start || empty_line_in_selection
}
