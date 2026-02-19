use gpui::{
    Bounds, Font, Hsla, Pixels, Point, ScrollStrategy, SharedString, Size, UniformListScrollHandle,
    Window, WrappedLine, px,
};
use smallvec::SmallVec;

use crate::input::{VisibleLineInfo, VisualLineInfo};
use crate::utils::{
    WIDTH_WRAP_BASE_MARGIN, build_visual_lines_from_wrap_boundaries,
    compute_max_visual_line_width, create_text_run, multiline_height,
};

/// Shapes text with wrapping and builds visual line info.
///
/// Returns `(wrapped_lines, visual_lines)`. Callers are responsible for storing these
/// and doing any additional bookkeeping (e.g. computing max line width).
pub fn shape_and_build_visual_lines(
    text: &SharedString,
    width: Pixels,
    font_size: Pixels,
    font: Font,
    text_color: Hsla,
    window: &Window,
) -> (Vec<WrappedLine>, Vec<VisualLineInfo>) {
    let mut visual_lines = Vec::new();

    if text.is_empty() {
        visual_lines.push(VisualLineInfo {
            start_offset: 0,
            end_offset: 0,
            wrapped_line_index: 0,
            visual_index_in_wrapped: 0,
        });
        return (Vec::new(), visual_lines);
    }

    let run = create_text_run(font, text_color, text.len());

    let wrapped_lines: SmallVec<[WrappedLine; 1]> = window
        .text_system()
        .shape_text(text.clone(), font_size, &[run], Some(width), None)
        .unwrap_or_default();

    let mut text_offset = 0;
    for (wrapped_idx, wrapped_line) in wrapped_lines.iter().enumerate() {
        let line_len = wrapped_line.len();
        build_visual_lines_from_wrap_boundaries(
            &mut visual_lines,
            wrapped_line,
            wrapped_idx,
            text_offset,
            line_len,
        );
        text_offset += line_len + 1;
    }

    (wrapped_lines.into_vec(), visual_lines)
}

/// Result of measuring wrapped text layout.
#[allow(dead_code)]
pub struct WrappedMeasureResult {
    /// The wrapped line objects from text shaping.
    pub wrapped_lines: Vec<WrappedLine>,
    /// Visual line info mapping visual lines to byte ranges.
    pub visual_lines: Vec<VisualLineInfo>,
    /// The maximum unwrapped line width across all lines.
    pub max_line_width: Pixels,
    /// The total number of visual lines (minimum 1).
    pub visual_line_count: usize,
    /// The computed element size (width, height).
    pub size: Size<Pixels>,
}

/// Measures wrapped text layout: shapes text, wraps at the given width, and computes
/// the element size based on visible lines and multiline clamp.
///
/// Used by both `WrappedTextElement` and `WrappedTextInputElement` measure callbacks
/// to avoid duplicating the core wrapping + sizing logic.
#[allow(dead_code)]
pub fn measure_wrapped_text(
    width: Pixels,
    line_height: Pixels,
    font_size: Pixels,
    font: Font,
    text_color: Hsla,
    text: &SharedString,
    multiline_clamp: Option<usize>,
    scale_factor: f32,
    known_width: bool,
    window: &Window,
) -> WrappedMeasureResult {
    let wrap_width = width + WIDTH_WRAP_BASE_MARGIN;

    let (wrapped_lines, visual_lines) =
        shape_and_build_visual_lines(text, wrap_width, font_size, font, text_color, window);

    let visual_line_count = visual_lines.len().max(1);

    let max_line_width = compute_max_visual_line_width(&wrapped_lines);

    let visible_lines = multiline_clamp
        .map_or(1, |c| c.min(visual_line_count))
        .max(1);
    let height = multiline_height(line_height, visible_lines, scale_factor);

    let result_width = if known_width {
        width
    } else {
        use crate::extensions::WindowExt;
        let content_width = window.round(max_line_width) + WIDTH_WRAP_BASE_MARGIN;
        content_width.min(width)
    };

    WrappedMeasureResult {
        wrapped_lines,
        visual_lines,
        max_line_width,
        visual_line_count,
        size: gpui::size(result_width, height),
    }
}

/// Scrolls a uniform_list to make the cursor visible.
///
/// Shared logic for both `InputState` and `SelectableTextState`.
pub fn ensure_cursor_visible_in_scroll(
    cursor_offset: usize,
    is_wrapped: bool,
    precomputed_visual_lines: &[VisualLineInfo],
    multiline_clamp: Option<usize>,
    scroll_handle: &UniformListScrollHandle,
    offset_to_line: impl FnOnce(usize) -> usize,
    line_count: impl FnOnce() -> usize,
) {
    let (target_line, total_lines) = if is_wrapped {
        let visual_line = precomputed_visual_lines
            .iter()
            .position(|info| cursor_offset >= info.start_offset && cursor_offset <= info.end_offset)
            .unwrap_or(0);
        (visual_line, precomputed_visual_lines.len())
    } else {
        let cursor_line = offset_to_line(cursor_offset);
        (cursor_line, line_count())
    };

    if multiline_clamp.map_or(false, |clamp| total_lines > clamp) {
        scroll_handle.scroll_to_item(target_line, ScrollStrategy::Center);
    }
}

/// Computes the text offset for a mouse position in multiline mode.
///
/// `horizontal_scroll_offset` is used to adjust for horizontal scrolling in non-wrapped mode
/// (Input uses its scroll offset; SelectableText passes `Pixels::ZERO`).
pub fn index_for_multiline_position(
    position: Point<Pixels>,
    line_height: Pixels,
    is_wrapped: bool,
    horizontal_scroll_offset: Pixels,
    visible_lines_info: &[VisibleLineInfo],
    precomputed_visual_lines: &[VisualLineInfo],
    last_bounds: Option<&Bounds<Pixels>>,
    line_start_offset: impl Fn(usize) -> usize,
    line_end_offset: impl Fn(usize) -> usize,
    line_count: impl FnOnce() -> usize,
) -> usize {
    // First try to find exact line from visible_lines_info
    if !visible_lines_info.is_empty() {
        for info in visible_lines_info {
            if info.bounds.contains(&position) {
                let local_x = if is_wrapped {
                    position.x - info.bounds.left()
                } else {
                    position.x - info.bounds.left() + horizontal_scroll_offset
                };
                let local_index = info.shaped_line.closest_index_for_x(local_x);

                if is_wrapped {
                    if let Some(visual_info) = precomputed_visual_lines.get(info.line_index) {
                        return visual_info.start_offset + local_index;
                    }
                }
                let ls = line_start_offset(info.line_index);
                return ls + local_index;
            }
        }

        // Check if above first visible line
        if let Some(first) = visible_lines_info.first() {
            if position.y < first.bounds.top() {
                let local_x = if is_wrapped {
                    position.x - first.bounds.left()
                } else {
                    position.x - first.bounds.left() + horizontal_scroll_offset
                };
                let local_index = first.shaped_line.closest_index_for_x(local_x);

                if is_wrapped {
                    if let Some(visual_info) = precomputed_visual_lines.get(first.line_index) {
                        if position.x < first.bounds.left() {
                            return visual_info.start_offset;
                        }
                        return visual_info.start_offset + local_index;
                    }
                }
                let ls = line_start_offset(first.line_index);
                if position.x < first.bounds.left() {
                    return ls;
                }
                return ls + local_index;
            }
        }

        // Check if below last visible line
        if let Some(last) = visible_lines_info.last() {
            if position.y >= last.bounds.bottom() {
                let local_x = if is_wrapped {
                    position.x - last.bounds.left()
                } else {
                    position.x - last.bounds.left() + horizontal_scroll_offset
                };
                let local_index = last.shaped_line.closest_index_for_x(local_x);

                if is_wrapped {
                    if let Some(visual_info) = precomputed_visual_lines.get(last.line_index) {
                        if position.x > last.bounds.right() {
                            return visual_info.end_offset;
                        }
                        return visual_info.start_offset + local_index;
                    }
                }
                let ls = line_start_offset(last.line_index);
                let le = line_end_offset(last.line_index);
                if position.x > last.bounds.right() {
                    return le;
                }
                return ls + local_index;
            }
        }

        // Handle horizontal overflow when Y is within line bounds
        for info in visible_lines_info {
            if position.y >= info.bounds.top() && position.y < info.bounds.bottom() {
                if is_wrapped {
                    if let Some(visual_info) = precomputed_visual_lines.get(info.line_index) {
                        if position.x < info.bounds.left() {
                            return visual_info.start_offset;
                        }
                        if position.x > info.bounds.right() {
                            return visual_info.end_offset;
                        }
                    }
                } else {
                    let ls = line_start_offset(info.line_index);
                    let le = line_end_offset(info.line_index);
                    if position.x < info.bounds.left() {
                        return ls;
                    }
                    if position.x > info.bounds.right() {
                        return le;
                    }
                }
            }
        }
    }

    // Fallback: estimate from position
    let Some(bounds) = last_bounds else {
        return 0;
    };

    let relative_y = position.y - bounds.top();
    let visible_line_index = if relative_y < px(0.) {
        0
    } else {
        (relative_y / line_height).floor() as usize
    };

    if is_wrapped {
        let visual_line_count = precomputed_visual_lines.len();
        let clamped_visual_line = visible_line_index.min(visual_line_count.saturating_sub(1));
        if let Some(visual_info) = precomputed_visual_lines.get(clamped_visual_line) {
            return visual_info.start_offset;
        }
    }

    let lc = line_count();
    let clamped_line = visible_line_index.min(lc.saturating_sub(1));
    line_start_offset(clamped_line)
}

/// Applies a selection change: sets the active end of the range and normalizes direction.
///
/// Shared core logic for `select_to_inner` in both Input and SelectableText.
pub fn apply_selection_change(
    selected_range: &mut std::ops::Range<usize>,
    selection_reversed: &mut bool,
    offset: usize,
) {
    if *selection_reversed {
        selected_range.start = offset;
    } else {
        selected_range.end = offset;
    }

    if selected_range.end < selected_range.start {
        *selection_reversed = !*selection_reversed;
        *selected_range = selected_range.end..selected_range.start;
    }
}
