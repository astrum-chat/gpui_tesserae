use std::ops::Range;

use gpui::{
    AbsoluteLength, Bounds, Font, Hsla, PaintQuad, Pixels, ShapedLine, TextRun,
    TextStyleRefinement, Window, WrappedLine, fill, point, px, size,
};

use crate::extensions::WindowExt;
use crate::utils::selection_shape::{
    SelectionShape, build_selection_primitive, compute_interior_corner_patches,
    compute_selection_corners, selection_config_from_options,
};

/// Maps visual line indices to byte ranges in the source text. Used by wrapped mode to translate between screen position and text offset.
#[derive(Clone, Debug)]
pub struct VisualLineInfo {
    /// Byte offset in the full text where this visual line starts.
    pub start_offset: usize,
    /// Byte offset in the full text where this visual line ends (exclusive).
    pub end_offset: usize,
    /// Index into the `WrappedLine` vec this segment belongs to.
    pub wrapped_line_index: usize,
    /// Which visual segment within the wrapped line (0 for first, increments at each wrap boundary).
    pub visual_index_in_wrapped: usize,
}

/// Information about a visible line in uniform_list mode (non-wrapped).
/// Used for accurate mouse hit testing.
#[derive(Clone)]
pub struct VisibleLineInfo {
    /// Absolute line index in the text
    pub line_index: usize,
    /// Screen bounds of this line element
    pub bounds: Bounds<Pixels>,
    /// Shaped line for X position lookup
    pub shaped_line: ShapedLine,
}

/// Base margin added to wrap widths to prevent janky text wrapping.
/// Added to fallback width estimates (when no cached container width is available)
/// and used as a threshold when comparing wrap widths for change detection.
pub const WIDTH_WRAP_BASE_MARGIN: Pixels = px(1.25);

/// Resolved text rendering parameters extracted from a `TextStyleRefinement` and window defaults.
pub struct TextRenderParams {
    /// The resolved font (family, weight, style, features, fallbacks).
    pub font: Font,
    /// The resolved font size in pixels.
    pub font_size: Pixels,
    /// The resolved line height, rounded to the pixel grid.
    pub line_height: Pixels,
    /// The window's display scale factor.
    pub scale_factor: f32,
}

/// Resolves font, font size, line height, and scale factor from a text style and window defaults.
pub fn compute_text_render_params(
    text_style: &TextStyleRefinement,
    window: &Window,
) -> TextRenderParams {
    let font_size = match text_style
        .font_size
        .unwrap_or_else(|| window.text_style().font_size)
    {
        AbsoluteLength::Pixels(px) => px,
        AbsoluteLength::Rems(rems) => rems.to_pixels(window.rem_size()),
    };
    let line_height = text_style
        .line_height
        .map(|lh| lh.to_pixels(font_size.into(), window.rem_size()))
        .unwrap_or_else(|| window.line_height());
    let line_height = window.round(line_height);
    let scale_factor = window.scale_factor();
    let font = Font {
        family: text_style
            .font_family
            .clone()
            .unwrap_or_else(|| window.text_style().font_family),
        features: text_style.font_features.clone().unwrap_or_default(),
        fallbacks: text_style.font_fallbacks.clone(),
        weight: text_style.font_weight.unwrap_or_default(),
        style: text_style.font_style.unwrap_or_default(),
    };
    TextRenderParams {
        font,
        font_size,
        line_height,
        scale_factor,
    }
}

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
    let selection_intersects = selected_range.start <= line_end && selected_range.end > line_start;

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
/// Returns the main selection shape plus any interior (concave) corner patches.
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
    multiline_wrapped: bool,
    selection_precise: bool,
    content_width: Option<Pixels>,
    window: &mut Window,
    corner_radius: Option<Pixels>,
    corner_smoothing: Option<f32>,
    prev_line_bounds: Option<(Pixels, Pixels)>,
    prev_line_end_offset: Option<usize>,
    next_line_bounds: Option<(Pixels, Pixels)>,
    next_line_end_offset: Option<usize>,
    debug_interior_corners: bool,
) -> Option<SelectionShape> {
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

    // Round scroll_offset to the pixel grid so that screen-space selection coordinates
    // (bounds.left() + end_x - scroll_offset) land on pixel boundaries.
    let scroll_offset = window.round(scroll_offset);

    // In wrapped mode (scroll_offset == 0), clamp selection to the container width
    // so the trailing-whitespace indicator fills to the edge but not beyond the clip.
    // In non-wrapped mode (scroll_offset > 0), only clamp the left edge to prevent
    // negative screen-space coordinates when a short line is scrolled past its content.
    // The content mask handles right-side clipping, so no max_x clamp is needed.
    let max_x = bounds.size.width + scroll_offset;

    // The edge to extend selection to, in text-space coordinates.
    // In non-wrapped mode, content_width is the full scrollable content width
    // (measured_max_line_width from state), so the selection fills the entire
    // scrollable area and the content_mask clips it at the viewport edge.
    // In wrapped mode, content_width is None and bounds.size.width IS the container.
    let edge_x = window.round(content_width.unwrap_or(bounds.size.width));

    if !selection_precise && selected_range.end > line_end {
        // Extend selection to the container edge, but only when the
        // selection continues past this line (not on the final selected line).
        selection_end_x = edge_x;
    } else if multiline_wrapped {
        // Clamp trailing whitespace to container edge
        if selection_end_x > max_x {
            selection_end_x = max_x;
        }
    }

    // Ensure the final selection end is pixel-aligned after all clamping/extending.
    selection_end_x = window.round(selection_end_x);

    // After clamping, selection may have become zero-width or inverted
    if selection_start_x >= selection_end_x {
        return None;
    }

    // Clamp adjacent line bounds consistently with the current line's treatment.
    // For selection_precise, only extend an adjacent line if the selection
    // continues past that line's end — matching what its own shape computation does.
    let clamp_adj = |b: Option<(Pixels, Pixels)>,
                     adj_end: Option<usize>|
     -> Option<(Pixels, Pixels)> {
        b.map(|(start, end)| {
            let adj_extends = !selection_precise && adj_end.is_some_and(|e| selected_range.end > e);
            if adj_extends {
                (start, edge_x)
            } else if multiline_wrapped {
                (start, end.min(max_x))
            } else {
                (start, end)
            }
        })
        .filter(|(start, end)| start < end)
    };
    let clamped_prev = clamp_adj(prev_line_bounds, prev_line_end_offset);
    let clamped_next = clamp_adj(next_line_bounds, next_line_end_offset);

    let config = selection_config_from_options(corner_radius, corner_smoothing);
    let corners = compute_selection_corners(
        selection_start_x,
        selection_end_x,
        clamped_prev,
        clamped_next,
        config.corner_radius,
        window.scale_factor(),
    );

    let shape = build_selection_primitive(
        bounds,
        selection_start_x,
        selection_end_x,
        scroll_offset,
        highlight_color,
        &config,
        corners,
    );

    let interior_corners = compute_interior_corner_patches(
        selection_start_x,
        selection_end_x,
        clamped_prev,
        clamped_next,
        config.corner_radius,
        config.corner_smoothing,
        window.scale_factor(),
        bounds.left(),
        bounds.top(),
        bounds.bottom(),
        bounds.size.height,
        scroll_offset,
        if debug_interior_corners {
            // DEBUG: red interior corners for visibility
            gpui::Hsla {
                h: 0.0,
                s: 1.0,
                l: 0.5,
                a: 1.0,
            }
        } else {
            highlight_color
        },
    );

    Some(SelectionShape::new(shape, interior_corners))
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

/// Shapes an adjacent line and computes its selection x-bounds.
/// Used to determine corner rounding for multi-line selections.
pub fn shape_and_compute_selection_bounds(
    full_value: &str,
    line_start: usize,
    line_end: usize,
    selected_range: &Range<usize>,
    font: &Font,
    font_size: Pixels,
    text_color: Hsla,
    window: &mut Window,
) -> Option<(Pixels, Pixels)> {
    let line_content = &full_value[line_start..line_end];
    let run = create_text_run(font.clone(), text_color, line_content.len());
    let shaped =
        window
            .text_system()
            .shape_line(line_content.to_string().into(), font_size, &[run], None);
    compute_selection_x_bounds(
        &shaped,
        selected_range,
        line_start,
        line_end,
        font,
        font_size,
        text_color,
        window,
    )
}

/// Computes selection x-bounds for the previous and next adjacent lines.
/// Returns `(None, None)` when `selection_rounded` is `None` (no corner rounding needed).
pub fn compute_adjacent_line_selection_bounds(
    full_value: &str,
    prev_offsets: Option<(usize, usize)>,
    next_offsets: Option<(usize, usize)>,
    selected_range: &Range<usize>,
    selection_rounded: Option<Pixels>,
    font: &Font,
    font_size: Pixels,
    text_color: Hsla,
    window: &mut Window,
) -> (Option<(Pixels, Pixels)>, Option<(Pixels, Pixels)>) {
    if selection_rounded.is_none() {
        return (None, None);
    }
    let prev_bounds = prev_offsets.and_then(|(start, end)| {
        shape_and_compute_selection_bounds(
            full_value,
            start,
            end,
            selected_range,
            font,
            font_size,
            text_color,
            window,
        )
    });
    let next_bounds = next_offsets.and_then(|(start, end)| {
        shape_and_compute_selection_bounds(
            full_value,
            start,
            end,
            selected_range,
            font,
            font_size,
            text_color,
            window,
        )
    });
    (prev_bounds, next_bounds)
}

/// Standard `request_layout` for a single line element: full width, fixed line height.
pub fn request_line_layout(
    line_height: Pixels,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> (gpui::LayoutId, ()) {
    let mut style = gpui::Style::default();
    style.size.width = gpui::relative(1.).into();
    style.size.height = line_height.into();
    (window.request_layout(style, [], cx), ())
}

/// Computes `(start_offset, end_offset)` pairs for each logical line in the text.
pub fn compute_line_offsets(text: &str) -> Vec<(usize, usize)> {
    text.split('\n')
        .scan(0, |start, line| {
            let end = *start + line.len();
            let offsets = (*start, end);
            *start = end + 1;
            Some(offsets)
        })
        .collect()
}

/// Computes the maximum visual line width across all wrapped visual lines.
/// Each visual line's width is measured using x_for_index on the unwrapped layout,
/// converting from the visual line's byte offsets to local offsets within its logical line.
pub fn compute_max_visual_line_width(
    visual_lines: &[VisualLineInfo],
    wrapped_lines: &[WrappedLine],
    text: &str,
) -> Pixels {
    let line_starts: Vec<usize> = {
        let mut starts = vec![0usize];
        for (i, b) in text.as_bytes().iter().enumerate() {
            if *b == b'\n' {
                starts.push(i + 1);
            }
        }
        starts
    };

    let mut max_width = Pixels::ZERO;
    for vl in visual_lines {
        let line_start = line_starts.get(vl.wrapped_line_index).copied().unwrap_or(0);
        let local_start = vl.start_offset.saturating_sub(line_start);
        let local_end = vl.end_offset.saturating_sub(line_start);
        if let Some(wl) = wrapped_lines.get(vl.wrapped_line_index) {
            let start_x = wl.unwrapped_layout.x_for_index(local_start);
            let end_x = wl.unwrapped_layout.x_for_index(local_end);
            let w = end_x - start_x;
            if w > max_width {
                max_width = w;
            }
        }
    }
    max_width
}

/// Determines if trailing whitespace should be shown in selection highlighting.
pub fn should_show_trailing_whitespace(
    selected_range: &Range<usize>,
    line_end_offset: usize,
) -> bool {
    selected_range.end > line_end_offset
}

/// Adjusts a horizontal scroll offset to keep `cursor_x` (in text-space) visible
/// within a container of `container_width`. Scroll speed scales with distance from
/// the edge and is normalized by delta time for frame-rate independence.
///
/// Returns the updated scroll offset.
pub fn auto_scroll_horizontal(
    horizontal_scroll_offset: Pixels,
    cursor_x: Pixels,
    container_width: Pixels,
    last_scroll_time: &mut Option<std::time::Instant>,
) -> Pixels {
    let now = std::time::Instant::now();
    let dt_secs = last_scroll_time
        .map(|t| now.duration_since(t).as_secs_f32())
        .unwrap_or(1.0 / 60.0)
        .min(0.1); // cap at 100ms to avoid huge jumps after pauses
    *last_scroll_time = Some(now);

    let scroll_margin = px(2.0);
    // pixels per second at 1px overshoot distance; scales linearly with distance
    let base_speed: f32 = 600.0;
    let accel: f32 = 3.0;

    let visible_start = horizontal_scroll_offset;
    let visible_end = horizontal_scroll_offset + container_width;
    let mut offset = horizontal_scroll_offset;

    if cursor_x < visible_start + scroll_margin {
        let overshoot = f32::from(visible_start + scroll_margin - cursor_x);
        let speed_px = base_speed + overshoot * accel;
        let max_step = px(speed_px * dt_secs);
        let delta = visible_start - (cursor_x - scroll_margin).max(Pixels::ZERO);
        offset -= delta.min(max_step);
    } else if cursor_x > visible_end - scroll_margin {
        let overshoot = f32::from(cursor_x - (visible_end - scroll_margin));
        let speed_px = base_speed + overshoot * accel;
        let max_step = px(speed_px * dt_secs);
        let target = cursor_x - container_width + scroll_margin;
        let delta = target - horizontal_scroll_offset;
        offset += delta.min(max_step);
    }

    offset
}

/// Clamps a vertical scroll offset to valid bounds based on the total number
/// of visual lines and the visible line count (from multiline_clamp).
pub fn clamp_vertical_scroll(
    scroll_offset: Pixels,
    line_height: Pixels,
    total_visual_lines: usize,
    multiline_clamp: Option<usize>,
) -> Pixels {
    let total = total_visual_lines.max(1);
    let visible = multiline_clamp.map_or(1, |c| c.min(total));
    let max_scroll = line_height * (total - visible) as f32;
    let max_scroll = if max_scroll > Pixels::ZERO {
        max_scroll
    } else {
        Pixels::ZERO
    };
    scroll_offset.max(Pixels::ZERO).min(max_scroll)
}

/// Adjusts vertical scroll offset to keep the cursor's visual line visible in wrapped mode.
/// Returns the updated scroll offset.
pub fn ensure_cursor_visible_wrapped(
    cursor_offset: usize,
    visual_lines: &[VisualLineInfo],
    line_height: Pixels,
    multiline_clamp: Option<usize>,
    scroll_offset: Pixels,
) -> Pixels {
    let visual_line = visual_lines
        .iter()
        .position(|info| cursor_offset >= info.start_offset && cursor_offset <= info.end_offset)
        .unwrap_or(0);

    let line_top = line_height * visual_line as f32;
    let line_bottom = line_top + line_height;
    let total_visual_lines = visual_lines.len().max(1);
    let visible_height =
        line_height * multiline_clamp.map_or(1, |c| c.min(total_visual_lines)) as f32;

    let mut offset = scroll_offset;
    if line_top < offset {
        offset = line_top;
    } else if line_bottom > offset + visible_height {
        offset = line_bottom - visible_height;
    }

    clamp_vertical_scroll(offset, line_height, total_visual_lines, multiline_clamp)
}

/// Computes the vertical auto-scroll throttle interval (in milliseconds) based on
/// how far outside the bounds the mouse is. Closer to the edge = slower, further = faster.
/// Returns `None` if the position is within bounds (no scroll needed).
pub fn auto_scroll_vertical_interval(
    position_y: Pixels,
    bounds_top: Pixels,
    bounds_bottom: Pixels,
) -> Option<u128> {
    let distance: f32 = if position_y < bounds_top {
        f32::from(bounds_top - position_y)
    } else if position_y > bounds_bottom {
        f32::from(position_y - bounds_bottom)
    } else {
        return None;
    };
    // 120ms when close, down to 20ms when far (200+ px away)
    Some((120.0_f32 - (distance * 0.5).min(100.0)).max(20.0) as u128)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::px;

    // ── compute_line_offsets ──────────────────────────────────────────

    #[test]
    fn test_line_offsets_single_line() {
        let offsets = compute_line_offsets("hello");
        assert_eq!(offsets, vec![(0, 5)]);
    }

    #[test]
    fn test_line_offsets_multiple_lines() {
        let offsets = compute_line_offsets("abc\ndef\nghi");
        assert_eq!(offsets, vec![(0, 3), (4, 7), (8, 11)]);
    }

    #[test]
    fn test_line_offsets_empty_string() {
        let offsets = compute_line_offsets("");
        assert_eq!(offsets, vec![(0, 0)]);
    }

    #[test]
    fn test_line_offsets_trailing_newline() {
        let offsets = compute_line_offsets("abc\n");
        assert_eq!(offsets, vec![(0, 3), (4, 4)]);
    }

    #[test]
    fn test_line_offsets_empty_lines() {
        let offsets = compute_line_offsets("a\n\nb");
        assert_eq!(offsets, vec![(0, 1), (2, 2), (3, 4)]);
    }

    #[test]
    fn test_line_offsets_unicode() {
        // "héllo\nwörld" — multi-byte chars
        let text = "héllo\nwörld";
        let offsets = compute_line_offsets(text);
        let first_line = &text[offsets[0].0..offsets[0].1];
        let second_line = &text[offsets[1].0..offsets[1].1];
        assert_eq!(first_line, "héllo");
        assert_eq!(second_line, "wörld");
    }

    // ── should_show_trailing_whitespace ───────────────────────────────

    #[test]
    fn test_trailing_ws_selection_past_line_end() {
        assert!(should_show_trailing_whitespace(&(0..10), 5));
    }

    #[test]
    fn test_trailing_ws_selection_at_line_end() {
        assert!(!should_show_trailing_whitespace(&(0..5), 5));
    }

    #[test]
    fn test_trailing_ws_selection_before_line_end() {
        assert!(!should_show_trailing_whitespace(&(0..3), 5));
    }

    #[test]
    fn test_trailing_ws_selection_exactly_at_end() {
        assert!(!should_show_trailing_whitespace(&(2..7), 7));
    }

    #[test]
    fn test_trailing_ws_selection_one_past_end() {
        assert!(should_show_trailing_whitespace(&(2..8), 7));
    }

    // ── multiline_height ─────────────────────────────────────────────

    #[test]
    fn test_multiline_height_basic() {
        let h = multiline_height(px(20.), 3, 2.0);
        assert_eq!(h, px(60.));
    }

    #[test]
    fn test_multiline_height_single_line() {
        let h = multiline_height(px(20.), 1, 2.0);
        assert_eq!(h, px(20.));
    }

    #[test]
    fn test_multiline_height_rounds_to_half_pixel_on_retina() {
        // 20.3 * 2 = 40.6, rounded to 0.5 increments: 40.5
        let h = multiline_height(px(20.3), 2, 2.0);
        assert_eq!(h, px(40.5));
    }

    #[test]
    fn test_multiline_height_rounds_to_whole_pixel_on_1x() {
        // 20.3 * 2 = 40.6, rounded to 1.0 increments: 41.0
        let h = multiline_height(px(20.3), 2, 1.0);
        assert_eq!(h, px(41.));
    }

    // ── auto_scroll_vertical_interval ─────────────────────────────────

    #[test]
    fn test_vertical_scroll_within_bounds_returns_none() {
        let result = auto_scroll_vertical_interval(px(50.), px(10.), px(100.));
        assert!(result.is_none());
    }

    #[test]
    fn test_vertical_scroll_above_bounds() {
        let result = auto_scroll_vertical_interval(px(5.), px(10.), px(100.));
        assert!(result.is_some());
    }

    #[test]
    fn test_vertical_scroll_below_bounds() {
        let result = auto_scroll_vertical_interval(px(110.), px(10.), px(100.));
        assert!(result.is_some());
    }

    #[test]
    fn test_vertical_scroll_closer_is_slower() {
        // 1px away should be slower (higher ms) than 100px away
        let close = auto_scroll_vertical_interval(px(9.), px(10.), px(100.)).unwrap();
        let far = auto_scroll_vertical_interval(px(-90.), px(10.), px(100.)).unwrap();
        assert!(close > far, "close={close} should be > far={far}");
    }

    #[test]
    fn test_vertical_scroll_minimum_interval() {
        // Very far away (500px) should still have at least 20ms
        let result = auto_scroll_vertical_interval(px(-490.), px(10.), px(100.)).unwrap();
        assert!(result >= 20);
    }

    #[test]
    fn test_vertical_scroll_at_bounds_edge() {
        // Exactly at top edge = within bounds
        assert!(auto_scroll_vertical_interval(px(10.), px(10.), px(100.)).is_none());
        // Exactly at bottom edge = within bounds
        assert!(auto_scroll_vertical_interval(px(100.), px(10.), px(100.)).is_none());
    }
}

#[cfg(all(test, feature = "test-support"))]
mod gpui_tests {
    use super::*;
    use crate::extensions::WindowExt;
    use gpui::{AppContext as _, Bounds, Hsla, TestAppContext, point, px, size};

    /// Helper: default black color.
    fn black() -> Hsla {
        Hsla {
            h: 0.,
            s: 0.,
            l: 0.,
            a: 1.,
        }
    }

    /// Helper: blue highlight color.
    fn highlight() -> Hsla {
        Hsla {
            h: 0.6,
            s: 1.,
            l: 0.5,
            a: 0.3,
        }
    }

    /// Helper: default test font (uses whatever the test platform provides).
    fn test_font() -> Font {
        Font {
            family: "Helvetica".into(),
            ..Default::default()
        }
    }

    /// Helper: shape a line of text and return (ShapedLine, Font, font_size).
    fn shape_text(text: &str, window: &mut Window) -> ShapedLine {
        let font = test_font();
        let font_size = px(14.);
        let run = create_text_run(font, black(), text.len());
        window
            .text_system()
            .shape_line(text.to_string().into(), font_size, &[run], None)
    }

    /// Helper: standard bounds for a line element.
    fn line_bounds(width: Pixels) -> Bounds<Pixels> {
        Bounds::new(point(px(0.), px(0.)), size(width, px(20.)))
    }

    // ── compute_selection_x_bounds ──────────────────────────────────

    #[gpui::test]
    fn test_selection_x_bounds_basic(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello world", window);
                let result = compute_selection_x_bounds(
                    &line,
                    &(0..5), // "hello"
                    0,
                    11,
                    &test_font(),
                    px(14.),
                    black(),
                    window,
                );
                assert!(result.is_some(), "should return bounds for valid selection");
                let (start_x, end_x) = result.unwrap();
                assert_eq!(start_x, px(0.), "selection starts at beginning");
                assert!(end_x > px(0.), "selection end should be positive");
            })
            .ok();
    }

    #[gpui::test]
    fn test_selection_x_bounds_empty_range(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello", window);
                let result = compute_selection_x_bounds(
                    &line,
                    &(3..3),
                    0,
                    5,
                    &test_font(),
                    px(14.),
                    black(),
                    window,
                );
                assert!(result.is_none(), "empty range should return None");
            })
            .ok();
    }

    #[gpui::test]
    fn test_selection_x_bounds_no_intersection(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello", window);
                // Selection is on a different line entirely
                let result = compute_selection_x_bounds(
                    &line,
                    &(10..20),
                    0,
                    5,
                    &test_font(),
                    px(14.),
                    black(),
                    window,
                );
                assert!(result.is_none(), "non-intersecting should return None");
            })
            .ok();
    }

    #[gpui::test]
    fn test_selection_x_bounds_trailing_whitespace(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window.update(cx, |_, window, _cx| {
            let line = shape_text("hello", window);
            // Selection extends past line end → trailing whitespace indicator
            let with_ws = compute_selection_x_bounds(
                &line, &(0..10), 0, 5, &test_font(), px(14.), black(), window,
            );
            // Selection ends at line end → no trailing whitespace
            let without_ws = compute_selection_x_bounds(
                &line, &(0..5), 0, 5, &test_font(), px(14.), black(), window,
            );
            assert!(with_ws.is_some());
            assert!(without_ws.is_some());
            let (_, end_with) = with_ws.unwrap();
            let (_, end_without) = without_ws.unwrap();
            assert!(
                end_with > end_without,
                "trailing whitespace should extend selection: with={end_with:?} without={end_without:?}"
            );
        }).ok();
    }

    #[gpui::test]
    fn test_selection_x_bounds_are_rounded(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello world test", window);
                let result = compute_selection_x_bounds(
                    &line,
                    &(0..16),
                    0,
                    16,
                    &test_font(),
                    px(14.),
                    black(),
                    window,
                );
                let (start_x, end_x) = result.unwrap();
                // Values should be rounded to pixel grid (0.5 increments on 2x, 1.0 on 1x)
                let scale = window.scale_factor();
                let increment = if scale >= 2.0 { 0.5 } else { 1.0 };
                let start_val = start_x.to_f64() as f32;
                let end_val = end_x.to_f64() as f32;
                assert_eq!(
                    (start_val / increment).round() * increment,
                    start_val,
                    "start_x should be pixel-aligned"
                );
                assert_eq!(
                    (end_val / increment).round() * increment,
                    end_val,
                    "end_x should be pixel-aligned"
                );
            })
            .ok();
    }

    // ── compute_selection_shape — extend-to-edge (default) ───────────

    #[gpui::test]
    fn test_selection_shape_extend_to_edge_default(cx: &mut TestAppContext) {
        // Default (selection_precise=false): selection on a non-last line extends to edge
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello", window);
                let bounds = line_bounds(px(300.));
                // selection 0..10 extends past line_end=5, so extend-to-edge triggers
                let shape = compute_selection_shape(
                    &line,
                    bounds,
                    &(0..10),
                    0,
                    5,
                    &test_font(),
                    px(14.),
                    black(),
                    highlight(),
                    px(0.), // scroll_offset
                    false,  // not wrapped
                    false,  // not precise (extend-to-edge)
                    None,   // no content_width → uses bounds.size.width
                    window,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                );
                assert!(shape.is_some(), "should produce a selection shape");
            })
            .ok();
    }

    #[gpui::test]
    fn test_selection_shape_extend_to_edge_not_on_last_line(cx: &mut TestAppContext) {
        // When selected_range.end <= line_end, extend-to-edge should NOT trigger
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello", window);
                let bounds = line_bounds(px(300.));
                // selection 0..5 ends exactly at line_end=5 → last line, no extension
                let shape = compute_selection_shape(
                    &line,
                    bounds,
                    &(0..5),
                    0,
                    5,
                    &test_font(),
                    px(14.),
                    black(),
                    highlight(),
                    px(0.),
                    false,
                    false, // not precise
                    None,
                    window,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                );
                assert!(shape.is_some());
            })
            .ok();
    }

    // ── compute_selection_shape — precise mode ───────────────────────

    #[gpui::test]
    fn test_selection_shape_precise_no_extension(cx: &mut TestAppContext) {
        // In precise mode, selection should NOT extend to edge even when
        // selected_range.end > line_end
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello", window);
                let bounds = line_bounds(px(300.));
                let shape = compute_selection_shape(
                    &line,
                    bounds,
                    &(0..10),
                    0,
                    5,
                    &test_font(),
                    px(14.),
                    black(),
                    highlight(),
                    px(0.),
                    false,
                    true, // precise mode
                    None,
                    window,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                );
                assert!(shape.is_some(), "precise mode should still produce a shape");
            })
            .ok();
    }

    // ── compute_selection_shape — content_width ──────────────────────

    #[gpui::test]
    fn test_selection_shape_with_content_width(cx: &mut TestAppContext) {
        // Non-wrapped mode with content_width: extend-to-edge should use content_width
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello", window);
                let bounds = line_bounds(px(300.));
                // bounds=300, content_width=800 (scrollable content wider than viewport)
                let shape = compute_selection_shape(
                    &line,
                    bounds,
                    &(0..10),
                    0,
                    5,
                    &test_font(),
                    px(14.),
                    black(),
                    highlight(),
                    px(100.),       // scroll_offset
                    false,          // not wrapped
                    false,          // not precise
                    Some(px(800.)), // content_width (scrollable content)
                    window,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                );
                assert!(shape.is_some());
            })
            .ok();
    }

    // ── scroll_offset rounding ───────────────────────────────────────

    #[gpui::test]
    fn test_selection_shape_scroll_offset_rounded(cx: &mut TestAppContext) {
        // The scroll_offset should be rounded inside compute_selection_shape
        // so screen-space coordinates are pixel-aligned
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let text = "a]b]c]d]e]f]g]h]i]j]k]l]m]n]o]p]q]r]s]t]u]v]w]x]y]z";
                let line = shape_text(text, window);
                let bounds = line_bounds(px(200.));
                // Use a sub-pixel scroll offset
                let shape = compute_selection_shape(
                    &line,
                    bounds,
                    &(0..text.len()),
                    0,
                    text.len(),
                    &test_font(),
                    px(14.),
                    black(),
                    highlight(),
                    px(50.37), // sub-pixel scroll offset
                    false,
                    true, // precise
                    None,
                    window,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                );
                assert!(
                    shape.is_some(),
                    "should produce shape with sub-pixel scroll"
                );
            })
            .ok();
    }

    // ── wrapped mode clamping ────────────────────────────────────────

    #[gpui::test]
    fn test_selection_shape_wrapped_clamps_to_container(cx: &mut TestAppContext) {
        // In wrapped mode, trailing whitespace should be clamped to container edge
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello", window);
                let bounds = line_bounds(px(300.));
                let shape = compute_selection_shape(
                    &line,
                    bounds,
                    &(0..10), // extends past line_end → trailing ws
                    0,
                    5,
                    &test_font(),
                    px(14.),
                    black(),
                    highlight(),
                    px(0.), // scroll_offset=0 in wrapped mode
                    true,   // wrapped
                    true,   // precise
                    None,
                    window,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    false,
                );
                assert!(shape.is_some());
            })
            .ok();
    }

    // ── adjacent line bounds in extend-to-edge mode ──────────────────

    #[gpui::test]
    fn test_selection_shape_adjacent_bounds_extended(cx: &mut TestAppContext) {
        // In extend-to-edge mode, adjacent line bounds should also be extended
        // when the selection continues past that adjacent line
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello", window);
                let bounds = line_bounds(px(300.));
                let shape = compute_selection_shape(
                    &line,
                    bounds,
                    &(0..20), // selection continues well past both lines
                    0,
                    5,
                    &test_font(),
                    px(14.),
                    black(),
                    highlight(),
                    px(0.),
                    false, // not wrapped
                    false, // not precise (extend-to-edge)
                    None,
                    window,
                    Some(px(4.)), // corner_radius
                    None,
                    Some((px(0.), px(30.))), // prev_line_bounds
                    Some(3),                 // prev_line_end_offset (selection continues past)
                    Some((px(0.), px(25.))), // next_line_bounds
                    Some(8),                 // next_line_end_offset (selection continues past)
                    false,
                );
                assert!(shape.is_some());
            })
            .ok();
    }

    #[gpui::test]
    fn test_selection_shape_adjacent_bounds_precise(cx: &mut TestAppContext) {
        // In precise mode, adjacent bounds should NOT be extended
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let line = shape_text("hello", window);
                let bounds = line_bounds(px(300.));
                let shape = compute_selection_shape(
                    &line,
                    bounds,
                    &(0..20),
                    0,
                    5,
                    &test_font(),
                    px(14.),
                    black(),
                    highlight(),
                    px(0.),
                    false,
                    true, // precise — adjacent bounds should stay as-is
                    None,
                    window,
                    Some(px(4.)),
                    None,
                    Some((px(0.), px(30.))),
                    Some(3),
                    Some((px(0.), px(25.))),
                    Some(8),
                    false,
                );
                assert!(shape.is_some());
            })
            .ok();
    }

    // ── shape_and_compute_selection_bounds ────────────────────────────

    #[gpui::test]
    fn test_shape_and_compute_bounds_basic(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let result = shape_and_compute_selection_bounds(
                    "hello world",
                    0,
                    11,
                    &(0..5),
                    &test_font(),
                    px(14.),
                    black(),
                    window,
                );
                assert!(result.is_some());
                let (start, end) = result.unwrap();
                assert_eq!(start, px(0.));
                assert!(end > px(0.));
            })
            .ok();
    }

    #[gpui::test]
    fn test_shape_and_compute_bounds_no_intersection(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let result = shape_and_compute_selection_bounds(
                    "hello world",
                    0,
                    11,
                    &(20..30), // doesn't intersect
                    &test_font(),
                    px(14.),
                    black(),
                    window,
                );
                assert!(result.is_none());
            })
            .ok();
    }

    // ── compute_adjacent_line_selection_bounds ────────────────────────

    #[gpui::test]
    fn test_adjacent_bounds_returns_none_without_rounding(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let (prev, next) = compute_adjacent_line_selection_bounds(
                    "line1\nline2\nline3",
                    Some((0, 5)),
                    Some((12, 17)),
                    &(0..17),
                    None, // no selection_rounded → should return (None, None)
                    &test_font(),
                    px(14.),
                    black(),
                    window,
                );
                assert!(prev.is_none());
                assert!(next.is_none());
            })
            .ok();
    }

    #[gpui::test]
    fn test_adjacent_bounds_with_rounding(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let (prev, next) = compute_adjacent_line_selection_bounds(
                    "line1\nline2\nline3",
                    Some((0, 5)),
                    Some((12, 17)),
                    &(0..17),
                    Some(px(4.)), // has rounding → should compute bounds
                    &test_font(),
                    px(14.),
                    black(),
                    window,
                );
                assert!(prev.is_some(), "prev should have bounds");
                assert!(next.is_some(), "next should have bounds");
            })
            .ok();
    }

    // ── window.round consistency ─────────────────────────────────────

    #[gpui::test]
    fn test_window_round_half_pixel_on_retina(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |_, cx| cx.new(|_| gpui::Empty))
                .unwrap()
        });
        window
            .update(cx, |_, window, _cx| {
                let scale = window.scale_factor();
                if scale >= 2.0 {
                    // On 2x displays, rounds to 0.5px increments
                    assert_eq!(window.round(px(10.3)), px(10.5));
                    assert_eq!(window.round(px(10.7)), px(10.5));
                    assert_eq!(window.round(px(10.0)), px(10.0));
                    assert_eq!(window.round(px(10.25)), px(10.5));
                    assert_eq!(window.round(px(10.75)), px(11.0));
                } else {
                    // On 1x displays, rounds to 1.0px increments
                    assert_eq!(window.round(px(10.3)), px(10.0));
                    assert_eq!(window.round(px(10.7)), px(11.0));
                    assert_eq!(window.round(px(10.0)), px(10.0));
                }
            })
            .ok();
    }
}
