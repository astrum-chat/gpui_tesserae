use std::ops::Range;

use gpui::{
    Bounds, Font, Hsla, PaintQuad, Pixels, ShapedLine, TextRun, Window, WrappedLine, fill, point,
    px, size,
};

use crate::input::VisualLineInfo;

use crate::extensions::WindowExt;

/// Base margin added to wrap widths to prevent janky text wrapping.
/// Added to fallback width estimates (when no cached container width is available)
/// and used as a threshold when comparing wrap widths for change detection.
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

/// Creates a simple `TextRun` with the given font, color, and length.
pub fn create_text_run(font: Font, color: Hsla, len: usize) -> TextRun {
    TextRun {
        len,
        font,
        color,
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

/// Computes the selection x-bounds (start_x, end_x) for a line if the selection intersects it.
pub fn compute_selection_x_bounds(
    line: &ShapedLine,
    selected_range: &Range<usize>,
    line_start: usize,
    line_end: usize,
    font: &Font,
    font_size: Pixels,
    text_color: Hsla,
    window: &mut Window,
) -> Option<(Pixels, Pixels)> {
    let selection_intersects = selected_range.start <= line_end && selected_range.end >= line_start;

    if selected_range.is_empty() || !selection_intersects {
        return None;
    }

    let line_len = line_end - line_start;
    let local_start = selected_range
        .start
        .saturating_sub(line_start)
        .min(line_len);
    let local_end = selected_range.end.saturating_sub(line_start).min(line_len);

    let mut selection_start_x = line.x_for_index(local_start);
    let mut selection_end_x = line.x_for_index(local_end);

    if should_show_trailing_whitespace(selected_range, line_end) {
        let space_run = create_text_run(font.clone(), text_color, 1);
        let space_line = window
            .text_system()
            .shape_line(" ".into(), font_size, &[space_run], None);
        selection_end_x = selection_end_x + space_line.x_for_index(1);
    }

    selection_start_x = window.round(selection_start_x);
    selection_end_x = window.round(selection_end_x);

    // A zero-width selection (e.g. selection ends exactly at the start of this line)
    // has no visual presence and should not affect adjacent line corner rounding.
    if selection_start_x == selection_end_x {
        return None;
    }

    Some((selection_start_x, selection_end_x))
}

/// Computes a full selection shape for a line, combining x-bounds, corner radius, and shape building.
pub fn compute_selection_shape(
    line: &ShapedLine,
    bounds: Bounds<Pixels>,
    selected_range: &Range<usize>,
    line_start: usize,
    line_end: usize,
    font: &Font,
    font_size: Pixels,
    text_color: Hsla,
    highlight_color: Hsla,
    scroll_offset: Pixels,
    window: &mut Window,
    corner_radius: Option<Pixels>,
    corner_smoothing: Option<f32>,
    prev_line_bounds: Option<(Pixels, Pixels)>,
    next_line_bounds: Option<(Pixels, Pixels)>,
) -> Option<super::SelectionShape> {
    let (selection_start_x, mut selection_end_x) = compute_selection_x_bounds(
        line,
        selected_range,
        line_start,
        line_end,
        font,
        font_size,
        text_color,
        window,
    )?;

    // Clamp selection to the available container width so the trailing-whitespace
    // indicator fills to the edge rather than painting beyond the clip boundary.
    let max_x = bounds.size.width - scroll_offset;
    if selection_end_x > max_x {
        selection_end_x = max_x;
    }

    let config = super::selection_config_from_options(corner_radius, corner_smoothing);
    let corners = super::compute_selection_corners(
        selection_start_x,
        selection_end_x,
        prev_line_bounds,
        next_line_bounds,
        config.corner_radius,
    );

    Some(super::build_selection_shape(
        bounds,
        selection_start_x,
        selection_end_x,
        scroll_offset,
        highlight_color,
        &config,
        corners,
    ))
}

/// Builds visual line info from a single wrapped line's wrap boundaries.
/// Handles both the no-wrap case (single visual line) and the wrapped case (multiple segments).
pub fn build_visual_lines_from_wrap_boundaries(
    visual_lines: &mut Vec<VisualLineInfo>,
    wrapped_line: &WrappedLine,
    wrapped_idx: usize,
    text_offset: usize,
    line_len: usize,
) {
    let wrap_boundaries = &wrapped_line.wrap_boundaries;

    if wrap_boundaries.is_empty() {
        visual_lines.push(VisualLineInfo {
            start_offset: text_offset,
            end_offset: text_offset + line_len,
            wrapped_line_index: wrapped_idx,
            visual_index_in_wrapped: 0,
        });
        return;
    }

    let mut segment_start = 0;
    for (visual_idx, boundary) in wrap_boundaries.iter().enumerate() {
        let run = &wrapped_line.unwrapped_layout.runs[boundary.run_ix];
        let glyph = &run.glyphs[boundary.glyph_ix];
        let segment_end = glyph.index;

        visual_lines.push(VisualLineInfo {
            start_offset: text_offset + segment_start,
            end_offset: text_offset + segment_end,
            wrapped_line_index: wrapped_idx,
            visual_index_in_wrapped: visual_idx,
        });
        segment_start = segment_end;
    }

    visual_lines.push(VisualLineInfo {
        start_offset: text_offset + segment_start,
        end_offset: text_offset + line_len,
        wrapped_line_index: wrapped_idx,
        visual_index_in_wrapped: wrap_boundaries.len(),
    });
}

/// Determines if trailing whitespace should be shown in selection highlighting.
pub fn should_show_trailing_whitespace(
    selected_range: &Range<usize>,
    line_end_offset: usize,
) -> bool {
    selected_range.end > line_end_offset
}
