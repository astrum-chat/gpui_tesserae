use std::ops::Range;

use gpui::{
    App, Bounds, ClipboardItem, Context, FocusHandle, Focusable, Font, Hsla, IntoElement, Pixels,
    Render, ScrollStrategy, SharedString, UniformListScrollHandle, Window, WrappedLine, div,
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
pub struct SelectableTextState {
    /// Handle for keyboard focus management.
    pub focus_handle: FocusHandle,
    text: SharedString,
    /// Byte range of the current selection. Empty range means cursor position only.
    pub selected_range: Range<usize>,
    /// If true, the cursor is at selection start; if false, at selection end.
    pub selection_reversed: bool,
    /// True while the user is dragging to select text.
    pub is_selecting: bool,

    /// Scroll handle for uniform_list.
    pub scroll_handle: UniformListScrollHandle,
    pub(crate) line_clamp: usize,
    pub(crate) is_wrapped: bool,
    pub(crate) line_height: Option<Pixels>,

    pub(crate) cached_wrap_width: Option<Pixels>,
    pub(crate) precomputed_visual_lines: Vec<VisualLineInfo>,
    pub(crate) precomputed_wrapped_lines: Vec<WrappedLine>,
    pub(crate) precomputed_at_width: Option<Pixels>,
    pub(crate) using_auto_width: bool,
    pub(crate) needs_wrap_recompute: bool,
    pub(crate) scroll_to_cursor_on_next_render: bool,

    pub(crate) visible_lines_info: Vec<VisibleLineInfo>,
    pub(crate) last_bounds: Option<Bounds<Pixels>>,
    /// Whether the current selection is a "select all" (cmd+a).
    pub is_select_all: bool,
    pub(crate) measured_max_line_width: Option<Pixels>,
    pub(crate) whitespace_width: Option<Pixels>,
    pub(crate) is_constrained: bool,
    /// Tracks previous focus state to detect blur events.
    was_focused: bool,
    /// Cached render params for use in paint phase
    pub(crate) last_font: Option<Font>,
    pub(crate) last_font_size: Option<Pixels>,
    pub(crate) last_text_color: Option<Hsla>,
    /// Pending mouse position for deferred selection processing.
    /// Stored during mouse move events and processed after visible_lines_info is populated.
    pub(crate) pending_selection_position: Option<gpui::Point<Pixels>>,
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
            whitespace_width: None,
            is_constrained: false,
            was_focused: false,
            last_font: None,
            last_font_size: None,
            last_text_color: None,
            pending_selection_position: None,
        }
    }

    /// Updates focus state and clears selection when focus is lost.
    /// Call this during render to detect blur events.
    pub fn update_focus_state(&mut self, window: &Window) {
        let is_focused = self.focus_handle.is_focused(window);
        if is_focused != self.was_focused {
            self.was_focused = is_focused;
            if !is_focused && !self.selected_range.is_empty() {
                self.selected_range = 0..0;
                self.is_select_all = false;
            }
        }
    }

    /// Sets the text content, preserving selection if valid, and triggering recomputation.
    pub fn text(&mut self, text: impl Into<SharedString>) {
        self.text = text.into();
        let text_len = self.text.len();

        // Clamp selection to valid range within new text
        let start = self.selected_range.start.min(text_len);
        let end = self.selected_range.end.min(text_len);
        self.selected_range = start..end;

        // If selection is now invalid, reset it
        if self.selected_range.start > self.selected_range.end {
            self.selected_range = 0..0;
            self.selection_reversed = false;
        }

        self.precomputed_visual_lines.clear();
        self.precomputed_wrapped_lines.clear();
        self.needs_wrap_recompute = true;
        self.measured_max_line_width = None;
        self.whitespace_width = None;
        self.precomputed_at_width = None;
    }

    /// Returns the current text content.
    pub fn get_text(&self) -> SharedString {
        self.text.clone()
    }

    pub(crate) fn set_multiline_params(&mut self, line_height: Pixels, line_clamp: usize) {
        self.line_height = Some(line_height);
        self.line_clamp = line_clamp;
    }

    pub(crate) fn set_wrap_mode(&mut self, wrapped: bool) {
        if self.is_wrapped != wrapped {
            self.cached_wrap_width = None;
            self.precomputed_visual_lines.clear();
            self.precomputed_wrapped_lines.clear();
            self.precomputed_at_width = None;
            self.needs_wrap_recompute = true;
        }
        self.is_wrapped = wrapped;
    }

    /// Pre-computes visual line info for wrapped text. Called during render to prepare
    /// data for uniform_list. Returns the number of visual lines.
    pub(crate) fn precompute_wrapped_lines(
        &mut self,
        width: Pixels,
        font_size: Pixels,
        font: Font,
        text_color: Hsla,
        window: &Window,
    ) -> usize {
        let text = self.get_text();
        self.precomputed_at_width = Some(width);

        let (wrapped_lines, visual_lines) = crate::utils::shape_and_build_visual_lines(
            &text, width, font_size, font, text_color, window,
        );

        // SelectableText-specific: track max line width for auto-width sizing
        self.measured_max_line_width = Some(
            wrapped_lines
                .iter()
                .map(|line| line.unwrapped_layout.width)
                .fold(Pixels::ZERO, |a, b| if b > a { b } else { a }),
        );

        self.precomputed_visual_lines = visual_lines;
        self.precomputed_wrapped_lines = wrapped_lines;

        if self.scroll_to_cursor_on_next_render {
            self.scroll_to_cursor_on_next_render = false;
            self.ensure_cursor_visible();
        }

        self.precomputed_visual_lines.len().max(1)
    }

    /// Ensure the cursor is visible by scrolling if necessary.
    pub fn ensure_cursor_visible(&mut self) {
        crate::utils::ensure_cursor_visible_in_scroll(
            self.cursor_offset(),
            self.is_wrapped,
            &self.precomputed_visual_lines,
            self.line_clamp,
            &self.scroll_handle,
            |offset| self.offset_to_line_col(offset).0,
            || self.line_count(),
        );
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
        crate::utils::apply_selection_change(
            &mut self.selected_range,
            &mut self.selection_reversed,
            offset,
        );

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
        if self.get_text().is_empty() {
            return 0;
        }
        crate::utils::index_for_multiline_position(
            position,
            line_height,
            self.is_wrapped,
            Pixels::ZERO,
            &self.visible_lines_info,
            &self.precomputed_visual_lines,
            self.last_bounds.as_ref(),
            |idx| self.line_start_offset(idx),
            |idx| self.line_end_offset(idx),
            || self.line_count(),
        )
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
        self.pending_selection_position = None;
    }

    /// Handles mouse move: stores position for deferred selection processing.
    /// The actual selection update happens after visible_lines_info is populated.
    pub fn on_mouse_move(
        &mut self,
        event: &gpui::MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            self.pending_selection_position = Some(event.position);
            cx.notify();
        }
    }

    /// Processes any pending selection update.
    /// Called after visible_lines_info is fully populated during paint.
    pub fn process_pending_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(position) = self.pending_selection_position.take() {
            if self.is_selecting {
                if let Some(line_height) = self.line_height {
                    self.select_to_multiline(position, line_height, cx);
                }
            }
        }
    }

    /// Handles scroll wheel: stops propagation if there's scrollable content.
    pub fn on_scroll_wheel(
        &mut self,
        _event: &gpui::ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Check if there's vertical scrollable content (more lines than visible)
        let line_count = if self.is_wrapped {
            self.precomputed_visual_lines.len()
        } else {
            self.line_count()
        };
        let has_vertical_scroll = line_count > self.line_clamp;

        if has_vertical_scroll {
            cx.stop_propagation();
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
