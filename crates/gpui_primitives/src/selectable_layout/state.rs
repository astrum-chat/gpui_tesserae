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
    pub(crate) selection_anchor: usize,
    pub is_selecting: bool,
    /// When double-click-dragging, the initially selected word range.
    /// Drag extends selection in word-sized increments.
    pub(crate) selecting_word: Option<Range<usize>>,
    /// The combined text from all children.
    pub(crate) combined_text: SharedString,
    pub(crate) child_byte_offsets: Vec<usize>,
    pub(crate) total_text_len: usize,
    /// Visual lines from text wrapping (computed in measure callback, consumed by prepaint).
    pub(crate) precomputed_visual_lines: Vec<VisualLineInfo>,
    /// Visible line info from prepaint (shaped lines + screen bounds for hit-testing).
    pub(crate) visible_lines_info: Vec<VisibleLineInfo>,
    /// Per-visual-line segment layouts (child segments with x-offsets for padding).
    pub(crate) line_layouts: Vec<VisualLinePrepaint>,
    /// Per-line byte ranges `(start, end)` for hit-testing.
    pub(crate) line_byte_ranges: Vec<(usize, usize)>,
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
    pub fn byte_offset_for_position(&self, position: Point<Pixels>) -> usize {
        if self.visible_lines_info.is_empty() {
            return 0;
        }

        // Exact bounds match.
        for info in &self.visible_lines_info {
            if info.bounds.contains(&position) {
                return self.offset_for_line(info.line_index, position.x - info.bounds.left());
            }
        }

        // Above first visible line.
        if let Some(first) = self.visible_lines_info.first() {
            if position.y < first.bounds.top() {
                return self.resolve_x_for_line(first, position.x);
            }
        }

        // Below last visible line.
        if let Some(last) = self.visible_lines_info.last() {
            if position.y >= last.bounds.bottom() {
                return self.resolve_x_for_line(last, position.x);
            }
        }

        // Y-range match for positions outside container bounds horizontally (or in a gap).
        for info in &self.visible_lines_info {
            if position.y >= info.bounds.top() && position.y < info.bounds.bottom() {
                return self.resolve_x_for_line(info, position.x);
            }
        }

        0
    }

    /// Resolve a byte offset for a line given a screen x position,
    /// snapping to line start/end when x is outside the line bounds.
    fn resolve_x_for_line(&self, info: &VisibleLineInfo, x: Pixels) -> usize {
        if x < info.bounds.left() {
            self.line_byte_ranges
                .get(info.line_index)
                .map_or(0, |&(s, _)| s)
        } else if x > info.bounds.right() {
            self.line_byte_ranges
                .get(info.line_index)
                .map_or(0, |&(_, e)| e)
        } else {
            self.offset_for_line(info.line_index, x - info.bounds.left())
        }
    }

    /// Resolve a byte offset for a given line index and local x position.
    fn offset_for_line(&self, line_index: usize, local_x: Pixels) -> usize {
        let Some(layout) = self.line_layouts.get(line_index) else {
            return 0;
        };
        if layout.segments.is_empty() {
            // Gap line — return end of byte range (= start of next content line).
            return self
                .line_byte_ranges
                .get(line_index)
                .map_or(0, |&(_, end)| end);
        }
        let offset = Self::closest_byte_offset_for_x(&layout.segments, local_x);
        if let Some(&(line_start, line_end)) = self.line_byte_ranges.get(line_index) {
            offset.clamp(line_start, line_end)
        } else {
            offset
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

        if let Some(first) = segments.first() {
            if local_x <= first.x_offset {
                return first.byte_range.start;
            }
        }

        for (i, seg) in segments.iter().enumerate() {
            let seg_end_x = seg.x_offset + seg.shaped_line.width;

            if local_x >= seg.x_offset && local_x <= seg_end_x {
                let local_index = seg.shaped_line.closest_index_for_x(local_x - seg.x_offset);
                return seg.byte_range.start + local_index;
            }

            if let Some(next) = segments.get(i + 1) {
                if local_x > seg_end_x && local_x < next.x_offset {
                    let dist_to_end = local_x - seg_end_x;
                    let dist_to_next = next.x_offset - local_x;
                    return if dist_to_end <= dist_to_next {
                        seg.byte_range.end
                    } else {
                        next.byte_range.start
                    };
                }
            }
        }

        segments.last().map_or(0, |s| s.byte_range.end)
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
                    self.extend_selection(offset);
                } else {
                    self.selection_anchor = offset;
                    self.selected_range = offset..offset;
                }
                self.is_selecting = true;
                self.selecting_word = None;
            }
            2 => {
                let word_range = self.word_range_at(offset);
                if word_range.is_empty() {
                    self.selection_anchor = offset;
                    self.selected_range = offset..offset;
                } else {
                    self.selected_range = word_range.clone();
                    self.selection_anchor = word_range.start;
                    self.is_selecting = true;
                    self.selecting_word = Some(word_range);
                }
            }
            3 => {
                let line_range = self.line_range_at(offset);
                if line_range.is_empty() {
                    self.selection_anchor = offset;
                    self.selected_range = offset..offset;
                } else {
                    self.selected_range = line_range.clone();
                    self.selection_anchor = line_range.start;
                    self.is_selecting = true;
                    self.selecting_word = None;
                }
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

    /// Global mouse move handler — fires even when cursor leaves the element bounds.
    pub(crate) fn on_mouse_move_global(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        let offset = self.byte_offset_for_position(position);

        if let Some(word_range) = &self.selecting_word {
            let original_word = word_range.clone();
            let current_word = self.word_range_at(offset);
            self.selected_range = if offset >= original_word.start {
                original_word.start..current_word.end
            } else {
                current_word.start..original_word.end
            };
        } else {
            self.extend_selection(offset);
        }
        cx.notify();
    }

    /// Set `selected_range` to span from `selection_anchor` to `offset`.
    fn extend_selection(&mut self, offset: usize) {
        if offset < self.selection_anchor {
            self.selected_range = offset..self.selection_anchor;
        } else {
            self.selected_range = self.selection_anchor..offset;
        }
    }

    /// Find the word range containing the given byte offset.
    /// Returns an empty range for newline characters (break lines).
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

        if ch == '\n' {
            return offset..offset;
        }

        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';

        if is_word_char(ch) {
            self.expand_range(offset, is_word_char)
        } else if ch.is_whitespace() {
            self.expand_range(offset, |c| c.is_whitespace())
        } else {
            // Single punctuation character.
            let char_len = ch.len_utf8();
            if offset < len {
                offset..offset + char_len
            } else {
                let prev_len = text[..offset]
                    .chars()
                    .next_back()
                    .map_or(0, |c| c.len_utf8());
                (offset - prev_len)..offset
            }
        }
    }

    /// Expand a range around `offset` by scanning backward and forward
    /// while `predicate` holds for each character.
    fn expand_range(&self, offset: usize, predicate: impl Fn(char) -> bool) -> Range<usize> {
        let text = self.combined_text.as_ref();
        let mut start = offset;
        let mut end = offset;

        for (i, c) in text[..offset].char_indices().rev() {
            if predicate(c) {
                start = i;
            } else {
                break;
            }
        }
        for (i, c) in text[offset..].char_indices() {
            if predicate(c) {
                end = offset + i + c.len_utf8();
            } else {
                break;
            }
        }

        start..end
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
