use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, FocusHandle, Focusable, Font, Hsla, IntoElement, Pixels,
    Render, ScrollStrategy, SharedString, TextRun, UniformListScrollHandle, Window, WrappedLine,
    div,
};

use crate::utils::TextNavigation;

// Re-export VisualLineInfo and VisibleLineInfo from input module since they're the same structure
pub use crate::input::{VisibleLineInfo, VisualLineInfo};

mod actions {
    #![allow(missing_docs)]
    use gpui::actions;

    actions!(
        selectable_text,
        [
            Copy,
            SelectAll,
            Left,
            Right,
            Up,
            Down,
            SelectLeft,
            SelectRight,
            SelectUp,
            SelectDown,
            Home,
            End,
            MoveToStartOfLine,
            MoveToEndOfLine,
            SelectToStartOfLine,
            SelectToEndOfLine,
            MoveToStart,
            MoveToEnd,
            SelectToStart,
            SelectToEnd,
            MoveToPreviousWord,
            MoveToNextWord,
            SelectToPreviousWordStart,
            SelectToNextWordEnd,
        ]
    );
}
pub use actions::*;

/// Core state for selectable text, managing text content, selection, and scroll position.
/// Unlike InputState, this is read-only and does not support editing, undo/redo, or IME.
pub struct SelectableTextState {
    /// Handle for keyboard focus management.
    pub focus_handle: FocusHandle,
    /// The text content to display.
    text: SharedString,
    /// Byte range of the current selection. Empty range means cursor position only.
    pub selected_range: Range<usize>,
    /// If true, the cursor is at selection start; if false, at selection end.
    pub selection_reversed: bool,
    /// True while the user is dragging to select text.
    pub is_selecting: bool,

    // Multiline support
    /// Scroll handle for uniform_list
    pub scroll_handle: UniformListScrollHandle,
    /// Maximum visible lines (for scroll calculations)
    pub(crate) line_clamp: usize,
    /// Whether text wrapping is enabled
    pub(crate) is_wrapped: bool,
    /// Line height for multiline calculations (set during render)
    pub(crate) line_height: Option<Pixels>,

    // Wrapped line computation
    /// Cached container width for wrapped text calculations
    pub(crate) cached_wrap_width: Option<Pixels>,
    /// Pre-computed visual lines for wrapped uniform_list mode
    pub(crate) precomputed_visual_lines: Vec<VisualLineInfo>,
    /// Pre-computed wrapped lines (the actual WrappedLine objects)
    pub(crate) precomputed_wrapped_lines: Vec<WrappedLine>,
    /// Width that was used to compute current precomputed_visual_lines
    pub(crate) precomputed_at_width: Option<Pixels>,
    /// Whether we're currently using auto width (text fits) vs fill width (text exceeds available)
    /// This affects how prepaint interprets width changes
    pub(crate) using_auto_width: bool,
    /// Flag indicating visual lines need recompute due to width mismatch
    pub(crate) needs_wrap_recompute: bool,
    /// Flag to scroll cursor into view on next render
    pub(crate) scroll_to_cursor_on_next_render: bool,

    // Hit testing
    /// Visible line info for uniform_list mode - populated during paint
    pub(crate) visible_lines_info: Vec<VisibleLineInfo>,
    /// Cached bounds from last render
    pub(crate) last_bounds: Option<Bounds<Pixels>>,
    /// Whether the current selection is a "select all" (cmd+a)
    pub is_select_all: bool,
    /// Maximum measured line width across all logical lines (for w_auto support)
    pub(crate) measured_max_line_width: Option<Pixels>,
}

impl SelectableTextState {
    /// Creates a new selectable text state with default values and a fresh focus handle.
    pub fn new(cx: &App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            text: SharedString::default(),
            selected_range: 0..0,
            selection_reversed: false,
            is_selecting: false,
            scroll_handle: UniformListScrollHandle::new(),
            line_clamp: usize::MAX,
            is_wrapped: true,
            line_height: None,
            cached_wrap_width: None,
            precomputed_visual_lines: Vec::new(),
            precomputed_wrapped_lines: Vec::new(),
            precomputed_at_width: None,
            using_auto_width: false,
            needs_wrap_recompute: false,
            scroll_to_cursor_on_next_render: false,
            visible_lines_info: Vec::new(),
            last_bounds: None,
            is_select_all: false,
            measured_max_line_width: None,
        }
    }

    /// Sets the text content. Clears selection and triggers recomputation of wrapped lines.
    pub fn text(&mut self, text: impl Into<SharedString>) {
        self.text = text.into();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.precomputed_visual_lines.clear();
        self.precomputed_wrapped_lines.clear();
        self.needs_wrap_recompute = true;
        self.measured_max_line_width = None;
    }

    /// Returns the current text content.
    pub fn get_text(&self) -> SharedString {
        self.text.clone()
    }

    /// Set multiline mode parameters (called during render)
    pub(crate) fn set_multiline_params(&mut self, line_height: Pixels, line_clamp: usize) {
        self.line_height = Some(line_height);
        self.line_clamp = line_clamp;
    }

    /// Pre-compute visual line info for wrapped text.
    /// Called during render to prepare data for uniform_list.
    /// Returns the number of visual lines.
    pub(crate) fn precompute_wrapped_lines(
        &mut self,
        width: Pixels,
        font_size: Pixels,
        font: Font,
        text_color: Hsla,
        window: &Window,
    ) -> usize {
        let text = self.get_text();

        // Track width used for this computation (don't overwrite cached_wrap_width -
        // that's set by prepaint when it detects the actual parent constraint)
        self.precomputed_at_width = Some(width);

        // Clear previous data
        self.precomputed_visual_lines.clear();
        self.precomputed_wrapped_lines.clear();

        if text.is_empty() {
            // For empty text, create one visual line
            self.precomputed_visual_lines.push(VisualLineInfo {
                start_offset: 0,
                end_offset: 0,
                wrapped_line_index: 0,
                visual_index_in_wrapped: 0,
            });
            return 1;
        }

        // Shape text with wrapping
        let run = TextRun {
            len: text.len(),
            font,
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let wrapped_lines = window
            .text_system()
            .shape_text(text.clone(), font_size, &[run], Some(width), None)
            .unwrap_or_default();

        // Measure max unwrapped line width for w_auto support
        let mut max_unwrapped_width = Pixels::ZERO;
        for wrapped_line in wrapped_lines.iter() {
            let unwrapped_width = wrapped_line.unwrapped_layout.width;
            if unwrapped_width > max_unwrapped_width {
                max_unwrapped_width = unwrapped_width;
            }
        }
        self.measured_max_line_width = Some(max_unwrapped_width);

        // Build visual line info from wrap boundaries
        let mut text_offset = 0;

        for (wrapped_idx, wrapped_line) in wrapped_lines.iter().enumerate() {
            let line_len = wrapped_line.len();
            let wrap_boundaries = &wrapped_line.wrap_boundaries;

            if wrap_boundaries.is_empty() {
                // No wrapping within this line
                self.precomputed_visual_lines.push(VisualLineInfo {
                    start_offset: text_offset,
                    end_offset: text_offset + line_len,
                    wrapped_line_index: wrapped_idx,
                    visual_index_in_wrapped: 0,
                });
            } else {
                // Line has wrap boundaries - create visual line for each segment
                let mut segment_start = 0;
                for (visual_idx, boundary) in wrap_boundaries.iter().enumerate() {
                    let run = &wrapped_line.unwrapped_layout.runs[boundary.run_ix];
                    let glyph = &run.glyphs[boundary.glyph_ix];
                    let segment_end = glyph.index;

                    self.precomputed_visual_lines.push(VisualLineInfo {
                        start_offset: text_offset + segment_start,
                        end_offset: text_offset + segment_end,
                        wrapped_line_index: wrapped_idx,
                        visual_index_in_wrapped: visual_idx,
                    });
                    segment_start = segment_end;
                }
                // Add final segment after last wrap boundary
                self.precomputed_visual_lines.push(VisualLineInfo {
                    start_offset: text_offset + segment_start,
                    end_offset: text_offset + line_len,
                    wrapped_line_index: wrapped_idx,
                    visual_index_in_wrapped: wrap_boundaries.len(),
                });
            }

            // Account for newline character between logical lines
            text_offset += line_len + 1;
        }

        // Store the wrapped lines for rendering
        self.precomputed_wrapped_lines = wrapped_lines.into_vec();

        // Handle deferred scroll now that visual lines are computed
        if self.scroll_to_cursor_on_next_render {
            self.scroll_to_cursor_on_next_render = false;
            self.ensure_cursor_visible();
        }

        self.precomputed_visual_lines.len().max(1)
    }

    /// Ensure the cursor is visible by scrolling if necessary.
    pub fn ensure_cursor_visible(&mut self) {
        let cursor_offset = self.cursor_offset();

        let (target_line, total_lines) = if self.is_wrapped {
            // For wrapped mode, find which visual line the cursor is on
            let visual_line = self
                .precomputed_visual_lines
                .iter()
                .position(|info| {
                    cursor_offset >= info.start_offset && cursor_offset <= info.end_offset
                })
                .unwrap_or(0);
            (visual_line, self.precomputed_visual_lines.len())
        } else {
            // For non-wrapped mode, use logical line
            let cursor_line = self.offset_to_line_col(cursor_offset).0;
            (cursor_line, self.line_count())
        };

        if total_lines > self.line_clamp {
            self.scroll_handle
                .scroll_to_item(target_line, ScrollStrategy::Center);
        }
    }

    /// Returns the active end of the selection (where the cursor is rendered).
    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn select_to_inner(&mut self, offset: usize, scroll: bool, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };

        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }

        if scroll {
            if self.is_wrapped {
                self.scroll_to_cursor_on_next_render = true;
            } else {
                self.ensure_cursor_visible();
            }
        }

        cx.notify()
    }

    /// Extends the selection to the given offset, scrolling to keep the cursor visible.
    pub fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.is_select_all = false;
        self.select_to_inner(offset, true, cx)
    }

    /// Extends the selection to the given offset without scrolling.
    pub fn select_to_without_scroll(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.select_to_inner(offset, false, cx)
    }

    /// Selects the word at the given offset (used for double-click selection).
    pub fn select_word_at(&mut self, offset: usize, cx: &mut Context<Self>) {
        let start = self.word_start(offset);
        let end = self.word_end(start);
        self.selected_range = start..end;
        self.selection_reversed = false;
        cx.notify()
    }

    fn move_to_inner(&mut self, offset: usize, scroll: bool, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        if scroll {
            if self.is_wrapped {
                self.scroll_to_cursor_on_next_render = true;
            } else {
                self.ensure_cursor_visible();
            }
        }
        cx.notify()
    }

    /// Sets cursor position and clears selection, scrolling to keep cursor visible.
    pub fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.is_select_all = false;
        self.move_to_inner(offset, true, cx)
    }

    /// Sets cursor position without auto-scrolling.
    pub fn move_to_without_scroll(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.move_to_inner(offset, false, cx)
    }

    // Action handlers

    /// Copies selected text to clipboard. No-op if nothing selected.
    pub fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.get_text()[self.selected_range.clone()].to_string(),
            ));
        }
    }

    /// Selects all text without scrolling.
    pub fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.is_select_all = true;
        self.move_to_without_scroll(0, cx);
        self.select_to_without_scroll(self.get_text().len(), cx)
    }

    /// Collapses selection to its start/end boundary, or moves one grapheme if no selection.
    pub fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    /// Collapses selection to its start/end boundary, or moves one grapheme if no selection.
    pub fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    /// Extends selection by one grapheme left.
    pub fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    /// Extends selection by one grapheme right.
    pub fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

    /// Collapses selection or moves to same column on previous line.
    pub fn up(&mut self, _: &Up, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let (line, col) = self.offset_to_line_col(self.cursor_offset());
            if line > 0 {
                let new_offset = self.line_col_to_offset(line - 1, col);
                self.move_to(new_offset, cx);
            }
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    /// Collapses selection or moves to same column on next line.
    pub fn down(&mut self, _: &Down, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            let (line, col) = self.offset_to_line_col(self.cursor_offset());
            if line < self.line_count().saturating_sub(1) {
                let new_offset = self.line_col_to_offset(line + 1, col);
                self.move_to(new_offset, cx);
            }
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    /// Extends selection to same column on previous line.
    pub fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        let (line, col) = self.offset_to_line_col(self.cursor_offset());
        if line > 0 {
            let new_offset = self.line_col_to_offset(line - 1, col);
            self.select_to(new_offset, cx);
        }
    }

    /// Extends selection to same column on next line.
    pub fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        let (line, col) = self.offset_to_line_col(self.cursor_offset());
        if line < self.line_count().saturating_sub(1) {
            let new_offset = self.line_col_to_offset(line + 1, col);
            self.select_to(new_offset, cx);
        }
    }

    /// Moves cursor to start of text.
    pub fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    /// Moves cursor to end of text.
    pub fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.get_text().len(), cx);
    }

    /// Moves cursor to start of current line.
    pub fn move_to_start_of_line(
        &mut self,
        _: &MoveToStartOfLine,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (line, _) = self.offset_to_line_col(self.cursor_offset());
        let target = self.line_start_offset(line);
        self.move_to(target, cx);
    }

    /// Moves cursor to end of current line.
    pub fn move_to_end_of_line(
        &mut self,
        _: &MoveToEndOfLine,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (line, _) = self.offset_to_line_col(self.cursor_offset());
        let target = self.line_end_offset(line);
        self.move_to(target, cx);
    }

    /// Extends selection to start of current line.
    pub fn select_to_start_of_line(
        &mut self,
        _: &SelectToStartOfLine,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (line, _) = self.offset_to_line_col(self.cursor_offset());
        let target = self.line_start_offset(line);
        self.select_to(target, cx);
    }

    /// Extends selection to end of current line.
    pub fn select_to_end_of_line(
        &mut self,
        _: &SelectToEndOfLine,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (line, _) = self.offset_to_line_col(self.cursor_offset());
        let target = self.line_end_offset(line);
        self.select_to(target, cx);
    }

    /// Moves cursor to start of document.
    pub fn move_to_start(&mut self, _: &MoveToStart, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    /// Moves cursor to end of document.
    pub fn move_to_end(&mut self, _: &MoveToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.get_text().len(), cx);
    }

    /// Extends selection to start of document.
    pub fn select_to_start(&mut self, _: &SelectToStart, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(0, cx);
    }

    /// Extends selection to end of document.
    pub fn select_to_end(&mut self, _: &SelectToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.get_text().len(), cx);
    }

    /// Moves past whitespace/punctuation to start of previous word.
    pub fn move_to_previous_word(
        &mut self,
        _: &MoveToPreviousWord,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let offset = self.cursor_offset();
            let prev = self.previous_boundary(offset);
            let target = self.word_start(prev);
            self.move_to(target, cx);
        } else {
            self.move_to(self.selected_range.start, cx);
        }
    }

    /// Moves to end of current/next word.
    pub fn move_to_next_word(
        &mut self,
        _: &MoveToNextWord,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let offset = self.cursor_offset();
            let target = self.word_end(offset);
            self.move_to(target, cx);
        } else {
            self.move_to(self.selected_range.end, cx);
        }
    }

    /// Extends selection to start of previous word.
    pub fn select_to_previous_word_start(
        &mut self,
        _: &SelectToPreviousWordStart,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.cursor_offset();
        let prev = self.previous_boundary(offset);
        let target = self.word_start(prev);
        self.select_to(target, cx);
    }

    /// Extends selection to end of next word.
    pub fn select_to_next_word_end(
        &mut self,
        _: &SelectToNextWordEnd,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let offset = self.cursor_offset();
        let target = self.word_end(offset);
        self.select_to(target, cx);
    }

    // Mouse handling

    /// Converts a mouse position to a text offset for multiline mode.
    pub fn index_for_multiline_position(
        &self,
        position: gpui::Point<Pixels>,
        line_height: Pixels,
    ) -> usize {
        let text = self.get_text();
        if text.is_empty() {
            return 0;
        }

        // First try to find exact line from visible_lines_info
        if !self.visible_lines_info.is_empty() {
            for info in &self.visible_lines_info {
                if info.bounds.contains(&position) {
                    let local_x = position.x - info.bounds.left();
                    let local_index = info.shaped_line.closest_index_for_x(local_x);

                    if self.is_wrapped {
                        if let Some(visual_info) =
                            self.precomputed_visual_lines.get(info.line_index)
                        {
                            return visual_info.start_offset + local_index;
                        }
                    }
                    let line_start = self.line_start_offset(info.line_index);
                    return line_start + local_index;
                }
            }

            // Check if above first visible line
            if let Some(first) = self.visible_lines_info.first() {
                if position.y < first.bounds.top() {
                    let local_x = position.x - first.bounds.left();
                    let local_index = first.shaped_line.closest_index_for_x(local_x);

                    if self.is_wrapped {
                        if let Some(visual_info) =
                            self.precomputed_visual_lines.get(first.line_index)
                        {
                            if position.x < first.bounds.left() {
                                return visual_info.start_offset;
                            }
                            return visual_info.start_offset + local_index;
                        }
                    }
                    let line_start = self.line_start_offset(first.line_index);
                    if position.x < first.bounds.left() {
                        return line_start;
                    }
                    return line_start + local_index;
                }
            }

            // Check if below last visible line
            if let Some(last) = self.visible_lines_info.last() {
                if position.y >= last.bounds.bottom() {
                    let local_x = position.x - last.bounds.left();
                    let local_index = last.shaped_line.closest_index_for_x(local_x);

                    if self.is_wrapped {
                        if let Some(visual_info) =
                            self.precomputed_visual_lines.get(last.line_index)
                        {
                            if position.x > last.bounds.right() {
                                return visual_info.end_offset;
                            }
                            return visual_info.start_offset + local_index;
                        }
                    }
                    let line_start = self.line_start_offset(last.line_index);
                    let line_end = self.line_end_offset(last.line_index);
                    if position.x > last.bounds.right() {
                        return line_end;
                    }
                    return line_start + local_index;
                }
            }
        }

        // Fallback: estimate from position
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };

        let relative_y = position.y - bounds.top();
        let visible_line_index = if relative_y < gpui::px(0.) {
            0
        } else {
            (relative_y / line_height).floor() as usize
        };

        if self.is_wrapped {
            let visual_line_count = self.precomputed_visual_lines.len();
            let clamped_visual_line = visible_line_index.min(visual_line_count.saturating_sub(1));
            if let Some(visual_info) = self.precomputed_visual_lines.get(clamped_visual_line) {
                return visual_info.start_offset;
            }
        }

        let line_count = self.line_count();
        let clamped_line = visible_line_index.min(line_count.saturating_sub(1));
        self.line_start_offset(clamped_line)
    }

    /// Extends selection to a position in multiline mode, auto-scrolling when dragging past edges.
    pub fn select_to_multiline(
        &mut self,
        position: gpui::Point<Pixels>,
        line_height: Pixels,
        cx: &mut Context<Self>,
    ) {
        let offset = self.index_for_multiline_position(position, line_height);
        self.select_to(offset, cx);

        if self.is_selecting {
            if let Some(bounds) = &self.last_bounds {
                if position.y < bounds.top() {
                    self.scroll_up_one_line();
                } else if position.y > bounds.bottom() {
                    self.scroll_down_one_line();
                }
            }
        }
    }

    pub(crate) fn scroll_up_one_line(&self) {
        if let Some(first) = self.visible_lines_info.first() {
            if first.line_index > 0 {
                self.scroll_handle
                    .scroll_to_item(first.line_index - 1, ScrollStrategy::Top);
            }
        }
    }

    pub(crate) fn scroll_down_one_line(&self) {
        let line_count = if self.is_wrapped {
            self.precomputed_visual_lines.len()
        } else {
            self.line_count()
        };

        if let Some(last) = self.visible_lines_info.last() {
            if last.line_index + 1 < line_count {
                self.scroll_handle
                    .scroll_to_item(last.line_index + 1, ScrollStrategy::Bottom);
            }
        }
    }

    /// Handles mouse down: starts selection, supports click/double-click/triple-click and shift-extend.
    pub fn on_mouse_down(
        &mut self,
        event: &gpui::MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;

        let index = if let Some(line_height) = self.line_height {
            self.index_for_multiline_position(event.position, line_height)
        } else {
            0
        };

        if event.click_count >= 3 {
            // Select line at click position
            let (line_start, line_end) = self.line_range_at(index);
            self.move_to_without_scroll(line_start, cx);
            self.select_to_without_scroll(line_end, cx);
        } else if event.click_count == 2 {
            self.select_word_at(index, cx);
        } else if event.modifiers.shift {
            self.select_to(index, cx);
        } else {
            self.move_to(index, cx)
        }
    }

    /// Handles mouse up: ends the current selection drag.
    pub fn on_mouse_up(
        &mut self,
        _: &gpui::MouseUpEvent,
        _window: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.is_selecting = false;
    }

    /// Handles mouse move: extends selection while dragging.
    pub fn on_mouse_move(
        &mut self,
        event: &gpui::MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            if let Some(line_height) = self.line_height {
                self.select_to_multiline(event.position, line_height, cx);
            }
        }
    }
}

impl Render for SelectableTextState {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl Focusable for SelectableTextState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TextNavigation for SelectableTextState {
    fn value(&self) -> SharedString {
        self.text.clone()
    }
}
