//! State management for SelectableLayout — selection, visual line info, and mouse interaction.

use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, FocusHandle, Focusable, IntoElement, Pixels, Point,
    Render, SharedString, Window, div,
};

use crate::selectable_layout::VisualLinePrepaint;
use crate::utils::{VisibleLineInfo, VisualLineInfo};

mod actions {
    #![allow(missing_docs)]
    use gpui::actions;

    actions!(selectable_layout, [SelectAll, Copy]);
}
pub use actions::*;

/// Core state for the SelectableLayout component.
#[allow(missing_docs)]
pub struct SelectableLayoutState {
    pub focus_handle: FocusHandle,
    /// Selection range as byte offsets into the combined text.
    pub selected_range: Range<usize>,
    /// Anchor point for drag selection (byte offset of the initial click).
    pub(crate) selection_anchor: usize,
    pub is_selecting: bool,
    /// When double-click-dragging, the word range initially selected.
    /// Drag extends selection in word-sized increments.
    pub(crate) selecting_word: Option<Range<usize>>,
    /// The combined text from all children.
    pub(crate) combined_text: SharedString,
    /// Byte offset where each child's text starts in the combined text.
    pub(crate) child_byte_offsets: Vec<usize>,
    /// Total byte length of the combined text.
    pub(crate) total_text_len: usize,
    /// Visual lines from text wrapping (computed in measure callback, consumed by prepaint).
    pub(crate) precomputed_visual_lines: Vec<VisualLineInfo>,
    /// Visible line info from prepaint (shaped lines + screen bounds for hit-testing).
    pub(crate) visible_lines_info: Vec<VisibleLineInfo>,
    /// Per-visual-line segment layouts (child segments with x-offsets for padding).
    pub(crate) line_layouts: Vec<VisualLinePrepaint>,
    /// Per-line byte ranges (start_offset, end_offset) for hit-testing gap lines.
    pub(crate) line_byte_ranges: Vec<(usize, usize)>,
    /// Container bounds from last paint.
    pub(crate) last_bounds: Option<Bounds<Pixels>>,
    was_focused: bool,
}

#[allow(missing_docs)]
impl SelectableLayoutState {
    pub fn new(cx: &App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            selected_range: 0..0,
            selection_anchor: 0,
            is_selecting: false,
            selecting_word: None,
            combined_text: SharedString::default(),
            child_byte_offsets: Vec::new(),
            total_text_len: 0,
            precomputed_visual_lines: Vec::new(),
            visible_lines_info: Vec::new(),
            line_layouts: Vec::new(),
            line_byte_ranges: Vec::new(),
            last_bounds: None,
            was_focused: false,
        }
    }

    pub fn update_focus_state(&mut self, window: &Window) {
        let is_focused = self.focus_handle.is_focused(window);
        if is_focused != self.was_focused {
            self.was_focused = is_focused;
            if !is_focused && !self.selected_range.is_empty() {
                self.selected_range = 0..0;
            }
        }
    }

    /// Find the byte offset in the combined text closest to the given screen position.
    /// Uses segment layouts to account for decoration padding between children.
    pub fn byte_offset_for_position(&self, position: Point<Pixels>) -> usize {
        if self.visible_lines_info.is_empty() {
            return 0;
        }

        // Find the line whose bounds contain the position.
        let line_info = if let Some(info) = self
            .visible_lines_info
            .iter()
            .find(|info| info.bounds.contains(&position))
        {
            info
        } else if let Some(first) = self.visible_lines_info.first() {
            if position.y < first.bounds.top() {
                first
            } else if let Some(last) = self.visible_lines_info.last() {
                if position.y >= last.bounds.bottom() {
                    last
                } else {
                    // Position is between content lines (in a gap from line_break).
                    // Snap to the content line just above.
                    self.visible_lines_info
                        .iter()
                        .rev()
                        .find(|info| info.bounds.bottom() <= position.y)
                        .unwrap_or(first)
                }
            } else {
                first
            }
        } else {
            return 0;
        };

        let local_x = position.x - line_info.bounds.left();

        // Use segment layout for this line to account for padding.
        if let Some(layout) = self.line_layouts.get(line_info.line_index) {
            if layout.segments.is_empty() {
                // Gap line (from line_break) — return the end of this gap's
                // byte range, which equals the start of the next content line.
                // The preceding content line's range already extends to
                // include the flush \n, so selecting through a gap line from
                // above still works. Using `end` (instead of `start`) prevents
                // horizontal drags on a content line from jumping backward
                // into the gap line above.
                self.line_byte_ranges
                    .get(line_info.line_index)
                    .map_or(0, |&(_start, end)| end)
            } else {
                let offset = Self::closest_byte_offset_for_x(&layout.segments, local_x);
                // Clamp to this line's byte range so dragging past the left
                // edge doesn't jump into the previous line.
                if let Some(&(line_start, line_end)) =
                    self.line_byte_ranges.get(line_info.line_index)
                {
                    offset.clamp(line_start, line_end)
                } else {
                    offset
                }
            }
        } else {
            0
        }
    }

    /// Map a local x position to the closest byte offset using segment layouts.
    fn closest_byte_offset_for_x(
        segments: &[crate::selectable_layout::ChildSegment],
        local_x: Pixels,
    ) -> usize {
        if segments.is_empty() {
            return 0;
        }

        // Check if x is before the first segment.
        if let Some(first) = segments.first() {
            if local_x <= first.x_offset {
                return first.byte_range.start;
            }
        }

        // Find which segment (or gap) the x falls in.
        for (i, seg) in segments.iter().enumerate() {
            let seg_end_x = seg.x_offset + seg.shaped_line.width;

            if local_x >= seg.x_offset && local_x <= seg_end_x {
                // Inside this segment — use shaped line hit-testing.
                let seg_local_x = local_x - seg.x_offset;
                let local_index = seg.shaped_line.closest_index_for_x(seg_local_x);
                return seg.byte_range.start + local_index;
            }

            // In the gap between this segment and the next.
            if let Some(next) = segments.get(i + 1) {
                if local_x > seg_end_x && local_x < next.x_offset {
                    // Snap to the closer edge.
                    let dist_to_end = local_x - seg_end_x;
                    let dist_to_next = next.x_offset - local_x;
                    if dist_to_end <= dist_to_next {
                        return seg.byte_range.end;
                    } else {
                        return next.byte_range.start;
                    }
                }
            }
        }

        // Past the last segment.
        if let Some(last) = segments.last() {
            last.byte_range.end
        } else {
            0
        }
    }

    pub fn on_mouse_down(
        &mut self,
        event: &gpui::MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.focus_handle.focus(window, cx);
        let offset = self.byte_offset_for_position(event.position);

        match event.click_count {
            1 => {
                if event.modifiers.shift {
                    // Extend selection from anchor.
                    if offset < self.selection_anchor {
                        self.selected_range = offset..self.selection_anchor;
                    } else {
                        self.selected_range = self.selection_anchor..offset;
                    }
                } else {
                    self.selection_anchor = offset;
                    self.selected_range = offset..offset;
                }
                self.is_selecting = true;
                self.selecting_word = None;
            }
            2 => {
                // Double click — select word.
                let word_range = self.word_range_at(offset);
                self.selected_range = word_range.clone();
                self.selection_anchor = word_range.start;
                self.is_selecting = true;
                self.selecting_word = Some(word_range);
            }
            3 => {
                // Triple click — select line.
                let line_range = self.line_range_at(offset);
                self.selected_range = line_range.clone();
                self.selection_anchor = line_range.start;
                self.is_selecting = true;
                self.selecting_word = None;
            }
            _ => {}
        }
        cx.notify();
    }

    pub fn on_mouse_up(
        &mut self,
        _event: &gpui::MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = false;
        self.selecting_word = None;
        cx.notify();
    }

    /// Called from the global mouse event handler in SelectableLayoutElement::paint().
    /// Fires even when the cursor leaves the element/window bounds.
    pub(crate) fn on_mouse_move_global(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let offset = self.byte_offset_for_position(position);

        if let Some(word_range) = &self.selecting_word {
            // Word-selection mode (double-click drag): extend in word increments.
            let original_word = word_range.clone();
            let current_word = self.word_range_at(offset);

            if offset >= original_word.start {
                self.selected_range = original_word.start..current_word.end;
            } else {
                self.selected_range = current_word.start..original_word.end;
            }
        } else {
            // Normal character-level drag.
            if offset < self.selection_anchor {
                self.selected_range = offset..self.selection_anchor;
            } else {
                self.selected_range = self.selection_anchor..offset;
            }
        }
        cx.notify();
    }

    /// Find the word range containing the given byte offset.
    fn word_range_at(&self, offset: usize) -> Range<usize> {
        let text = self.combined_text.as_ref();
        let len = text.len();

        if len == 0 {
            return 0..0;
        }

        let offset = offset.min(len);

        let ch = if offset < len {
            text[offset..].chars().next().unwrap_or(' ')
        } else {
            text[..offset].chars().next_back().unwrap_or(' ')
        };

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        if is_word_char(ch) {
            let mut start = offset;
            let mut end = offset;

            for (i, c) in text[..offset].char_indices().rev() {
                if is_word_char(c) {
                    start = i;
                } else {
                    break;
                }
            }

            for (i, c) in text[offset..].char_indices() {
                if is_word_char(c) {
                    end = offset + i + c.len_utf8();
                } else {
                    break;
                }
            }

            start..end
        } else if ch.is_whitespace() {
            let mut start = offset;
            let mut end = offset;

            for (i, c) in text[..offset].char_indices().rev() {
                if c.is_whitespace() {
                    start = i;
                } else {
                    break;
                }
            }

            for (i, c) in text[offset..].char_indices() {
                if c.is_whitespace() {
                    end = offset + i + c.len_utf8();
                } else {
                    break;
                }
            }

            start..end
        } else {
            // Single punctuation character.
            let char_len = ch.len_utf8();
            if offset < len {
                offset..offset + char_len
            } else {
                (offset
                    - text[..offset]
                        .chars()
                        .next_back()
                        .map_or(0, |c| c.len_utf8()))..offset
            }
        }
    }

    /// Find the line range (between newlines) containing the given byte offset.
    fn line_range_at(&self, offset: usize) -> Range<usize> {
        let text = self.combined_text.as_ref();
        let len = text.len();

        if len == 0 {
            return 0..0;
        }

        let offset = offset.min(len);

        let start = text[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);

        let end = text[offset..].find('\n').map(|i| offset + i).unwrap_or(len);

        start..end
    }

    pub fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.selected_range = 0..self.total_text_len;
        cx.notify();
    }

    pub fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() && self.selected_range.end <= self.combined_text.len() {
            let text = &self.combined_text[self.selected_range.clone()];
            cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
        }
    }
}

impl Render for SelectableLayoutState {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl Focusable for SelectableLayoutState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
