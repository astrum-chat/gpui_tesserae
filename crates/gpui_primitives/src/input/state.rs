use std::ops::Range;
use std::sync::Arc;

use gpui::{
    App, AppContext as _, Bounds, ClipboardItem, Context, Entity, EntityInputHandler, FocusHandle,
    Focusable, Font, Hsla, IntoElement, Pixels, Render, ScrollWheelEvent, ShapedLine, SharedString,
    UTF16Selection, UniformListScrollHandle, Window, WrappedLine, div, point, px,
};

use crate::input::CursorBlink;
use crate::utils::TextNavigation;

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

/// Function type for transforming text when it changes.
/// Takes the full text after the change and returns the transformed text.
pub type MapTextFn = Arc<dyn Fn(SharedString) -> SharedString + Send + Sync>;

/// Entry in the undo/redo history stack.
#[derive(Clone)]
struct UndoEntry {
    text: SharedString,
    selected_range: Range<usize>,
    selection_reversed: bool,
}

mod actions {
    #![allow(missing_docs)]
    use gpui::actions;

    actions!(
        text_input,
        [
            Backspace,
            Delete,
            Left,
            Right,
            Up,
            Down,
            SelectLeft,
            SelectRight,
            SelectUp,
            SelectDown,
            SelectAll,
            Home,
            End,
            ShowCharacterPalette,
            Paste,
            Cut,
            Copy,
            Submit,
            SecondarySubmit,
            Quit,
            Undo,
            Redo,
            MoveToPreviousWord,
            MoveToNextWord,
            SelectToPreviousWordStart,
            SelectToNextWordEnd,
            MoveToStartOfLine,
            MoveToEndOfLine,
            SelectToStartOfLine,
            SelectToEndOfLine,
            MoveToStart,
            MoveToEnd,
            SelectToStart,
            SelectToEnd,
            DeleteToPreviousWordStart,
            DeleteToNextWordEnd,
            DeleteToBeginningOfLine,
            DeleteToEndOfLine,
        ]
    );
}
pub use actions::*;

/// Core state for a text input, managing text content, selection, cursor, and IME composition.
pub struct InputState {
    /// Handle for keyboard focus management.
    pub focus_handle: FocusHandle,
    /// The current text value, or None if empty.
    pub value: Option<SharedString>,
    /// Byte range of the current selection. Empty range means cursor position only.
    pub selected_range: Range<usize>,
    /// If true, the cursor is at selection start; if false, at selection end.
    pub selection_reversed: bool,
    /// Byte range of IME composition text (marked text), if any.
    pub marked_range: Option<Range<usize>>,
    /// Cached shaped line from last render (single-line mode).
    pub last_layout: Option<ShapedLine>,
    /// Cached bounds from last render.
    pub last_bounds: Option<Bounds<Pixels>>,
    /// True while the user is dragging to select text.
    pub is_selecting: bool,
    /// Entity managing cursor blink animation.
    pub cursor_blink: Entity<CursorBlink>,
    was_focused: bool,
    /// Whether the input is in multiline mode (set during render)
    pub(crate) is_multiline: bool,
    /// Line height for multiline calculations (set during render)
    pub(crate) line_height: Option<Pixels>,
    /// Closure to transform text when it changes (modifies stored value)
    pub(crate) map_text: Option<MapTextFn>,
    /// Whether the input is in wrapped mode (text wrapping enabled)
    pub(crate) is_wrapped: bool,
    /// Scroll handle for uniform_list (both wrapped and non-wrapped modes)
    pub scroll_handle: UniformListScrollHandle,
    /// Maximum visible lines (for scroll calculations)
    pub(crate) line_clamp: usize,
    /// Visible line info for uniform_list mode - populated during paint
    pub(crate) visible_lines_info: Vec<VisibleLineInfo>,
    /// Cached container width for wrapped text calculations (set during prepaint)
    pub(crate) cached_wrap_width: Option<Pixels>,
    /// Pre-computed visual lines for wrapped uniform_list mode
    pub(crate) precomputed_visual_lines: Vec<VisualLineInfo>,
    /// Pre-computed wrapped lines (the actual WrappedLine objects)
    pub(crate) precomputed_wrapped_lines: Vec<WrappedLine>,
    /// Width that was used to compute current precomputed_visual_lines
    pub(crate) precomputed_at_width: Option<Pixels>,

    /// Flag indicating visual lines need recompute due to width mismatch
    pub(crate) needs_wrap_recompute: bool,
    /// Flag to scroll cursor into view on next render (for wrapped mode)
    /// This defers scrolling until after visual lines are recomputed
    pub(crate) scroll_to_cursor_on_next_render: bool,

    /// Horizontal scroll offset for single-line mode (in pixels)
    pub(crate) horizontal_scroll_offset: Pixels,
    /// Vertical scroll offset for wrapped mode with line_clamp (in pixels)
    pub(crate) vertical_scroll_offset: Pixels,
    /// Last measured text width (for scroll wheel calculations)
    pub(crate) last_text_width: Pixels,
    /// When true, skip auto-scroll to cursor (user is manually scrolling)
    pub(crate) is_manually_scrolling: bool,
    /// When true, skip auto-scroll to cursor on next render (for select_all)
    pub(crate) skip_auto_scroll_on_next_render: bool,
    /// Whether the current selection is a "select all" (cmd+a)
    pub is_select_all: bool,

    /// Cached render params for use in prepaint-phase re-wrapping
    pub(crate) last_font: Option<Font>,
    pub(crate) last_font_size: Option<Pixels>,
    pub(crate) last_text_color: Option<Hsla>,

    /// Undo history stack
    undo_stack: Vec<UndoEntry>,
    /// Redo history stack
    redo_stack: Vec<UndoEntry>,
    /// Maximum number of undo/redo entries to keep
    max_history: usize,
}

impl InputState {
    /// Creates a new input state with default values and a fresh focus handle.
    pub fn new(cx: &mut App) -> Self {
        InputState {
            focus_handle: cx.focus_handle().tab_stop(true),
            value: None,
            selected_range: 0..0,
            selection_reversed: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
            is_selecting: false,
            cursor_blink: cx.new(|_| CursorBlink::new()),
            was_focused: false,
            is_multiline: false,
            line_height: None,
            map_text: None,
            is_wrapped: false,
            scroll_handle: UniformListScrollHandle::new(),
            line_clamp: 1,
            visible_lines_info: Vec::new(),
            cached_wrap_width: None,
            precomputed_visual_lines: Vec::new(),
            precomputed_wrapped_lines: Vec::new(),
            precomputed_at_width: None,

            needs_wrap_recompute: false,
            scroll_to_cursor_on_next_render: false,
            horizontal_scroll_offset: Pixels::ZERO,
            vertical_scroll_offset: Pixels::ZERO,
            last_text_width: Pixels::ZERO,
            is_manually_scrolling: false,
            skip_auto_scroll_on_next_render: false,
            is_select_all: false,
            last_font: None,
            last_font_size: None,
            last_text_color: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 200,
        }
    }

    /// Set the maximum number of undo/redo entries to keep.
    pub fn set_max_history(&mut self, max: usize) {
        self.max_history = max;
        // Trim existing stacks if they exceed the new limit
        if self.undo_stack.len() > max {
            self.undo_stack.drain(0..self.undo_stack.len() - max);
        }
        if self.redo_stack.len() > max {
            self.redo_stack.drain(0..self.redo_stack.len() - max);
        }
    }

    /// Set multiline mode parameters (called during render)
    pub(crate) fn set_multiline_params(
        &mut self,
        is_multiline: bool,
        line_height: Pixels,
        line_clamp: usize,
    ) {
        self.is_multiline = is_multiline;
        self.line_height = Some(line_height);
        self.line_clamp = line_clamp;
    }

    /// Pre-compute visual line info for wrapped text.
    /// Returns the number of visual lines.
    #[allow(dead_code)]
    pub(crate) fn precompute_wrapped_lines(
        &mut self,
        width: Pixels,
        font_size: Pixels,
        font: Font,
        text_color: Hsla,
        window: &Window,
    ) -> usize {
        let text = self.value();

        // Input-specific: update cached wrap width
        self.cached_wrap_width = Some(width);
        self.precomputed_at_width = Some(width);

        let (wrapped_lines, visual_lines) = crate::utils::shape_and_build_visual_lines(
            &text, width, font_size, font, text_color, window,
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
        if self.is_wrapped {
            // Pixel-based vertical scroll for wrapped mode
            let line_height = self.line_height.unwrap_or(px(16.0));
            let cursor = self.cursor_offset();
            let visual_line = self
                .precomputed_visual_lines
                .iter()
                .position(|info| cursor >= info.start_offset && cursor <= info.end_offset)
                .unwrap_or(0);

            let line_top = line_height * visual_line as f32;
            let line_bottom = line_top + line_height;
            let visible_height = line_height * self.line_clamp as f32;

            if line_top < self.vertical_scroll_offset {
                self.vertical_scroll_offset = line_top;
            } else if line_bottom > self.vertical_scroll_offset + visible_height {
                self.vertical_scroll_offset = line_bottom - visible_height;
            }

            self.clamp_vertical_scroll();
        } else {
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
    }

    /// Clamp vertical scroll offset to valid bounds.
    pub(crate) fn clamp_vertical_scroll(&mut self) {
        let line_height = self.line_height.unwrap_or(px(16.0));
        let total_lines = self.precomputed_visual_lines.len().max(1);
        let visible_lines = self.line_clamp.min(total_lines);
        let max_scroll = line_height * (total_lines - visible_lines) as f32;
        let max_scroll = if max_scroll > Pixels::ZERO {
            max_scroll
        } else {
            Pixels::ZERO
        };
        self.vertical_scroll_offset = self
            .vertical_scroll_offset
            .max(Pixels::ZERO)
            .min(max_scroll);
    }

    /// Ensure the cursor is horizontally visible in single-line or non-wrapped multiline mode.
    /// Called during prepaint with the cursor's x position and container width.
    /// Returns the updated scroll offset.
    pub(crate) fn ensure_cursor_visible_horizontal(
        &mut self,
        cursor_x: Pixels,
        container_width: Pixels,
    ) -> Pixels {
        // Don't scroll in wrapped mode (text wraps, so horizontal scroll not needed)
        if self.is_wrapped {
            return Pixels::ZERO;
        }

        // If user is manually scrolling, don't auto-scroll to cursor
        if self.is_manually_scrolling {
            return self.horizontal_scroll_offset;
        }

        let scroll_margin = px(2.0); // Small margin to keep cursor slightly away from edges
        let visible_start = self.horizontal_scroll_offset;
        let visible_end = self.horizontal_scroll_offset + container_width;

        if cursor_x < visible_start + scroll_margin {
            // Cursor is to the left of visible area - scroll left
            self.horizontal_scroll_offset = (cursor_x - scroll_margin).max(Pixels::ZERO);
        } else if cursor_x > visible_end - scroll_margin {
            // Cursor is to the right of visible area - scroll right
            self.horizontal_scroll_offset = cursor_x - container_width + scroll_margin;
        }

        self.horizontal_scroll_offset
    }

    /// Apply map_text transformation if set
    fn apply_map_text(&self, text: String) -> String {
        if let Some(map_fn) = &self.map_text {
            map_fn(text.into()).to_string()
        } else {
            text
        }
    }

    /// Call this during render to update focus state and manage cursor blink
    pub fn update_focus_state(&mut self, window: &Window, cx: &mut Context<Self>) {
        let is_focused = self.focus_handle.is_focused(window);
        if is_focused != self.was_focused {
            self.was_focused = is_focused;
            if is_focused {
                self.start_cursor_blink(cx);
            } else {
                self.stop_cursor_blink(cx);
                // Clear selection when blurred
                let cursor = self.cursor_offset();
                self.selected_range = cursor..cursor;
            }
        }
    }

    /// Returns whether the cursor should be rendered this frame (toggles for blink effect).
    pub fn cursor_visible(&self, cx: &App) -> bool {
        self.cursor_blink.read(cx).visible()
    }

    /// Starts cursor blinking. Called automatically when input gains focus.
    pub fn start_cursor_blink(&self, cx: &mut Context<Self>) {
        self.cursor_blink.update(cx, |blink, cx| {
            blink.start(cx);
        });
    }

    /// Stops cursor blinking, leaving it visible. Called automatically when input loses focus.
    pub fn stop_cursor_blink(&self, cx: &mut Context<Self>) {
        self.cursor_blink.update(cx, |blink, cx| {
            blink.stop();
            cx.notify();
        });
    }

    pub(crate) fn reset_cursor_blink(&self, cx: &mut Context<Self>) {
        self.cursor_blink.update(cx, |blink, cx| {
            blink.reset(cx);
        });
    }

    /// Returns the current text, or empty string if unset.
    pub fn value(&self) -> SharedString {
        self.value
            .clone()
            .unwrap_or_else(|| SharedString::new_static(""))
    }

    /// Takes and returns the current value, leaving the input empty.
    pub fn clear(&mut self) -> Option<SharedString> {
        self.selected_range = 0..0;
        self.value.take()
    }

    /// Builder method: sets initial text only if value is currently unset.
    pub fn initial_value(mut self, text: impl Into<SharedString>) -> Self {
        if self.value.is_some() {
            return self;
        };
        self.value = Some(text.into());
        self
    }

    // Action handlers for keyboard navigation and editing.
    // These are registered via `.on_action()` in the Input element's render method.

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

    /// Extends selection by one grapheme.
    pub fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    /// Extends selection by one grapheme.
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

    /// Inserts a newline.
    pub fn insert_newline(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(None, "\n", window, cx);
    }

    /// Inserts a newline.
    pub fn insert_newline_secondary(
        &mut self,
        _: &SecondarySubmit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(None, "\n", window, cx);
    }

    /// Selects all text without scrolling.
    pub fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.is_select_all = true;
        self.move_to_without_scroll(0, cx);
        self.select_to_without_scroll(self.value().len(), cx)
    }

    /// Moves cursor to start of text.
    pub fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    /// Moves cursor to end of text.
    pub fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.value().len(), cx);
    }

    // Word navigation (Option+Arrow on macOS, Ctrl+Arrow elsewhere)

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

    // Line navigation (Cmd+Arrow on macOS)

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

    // Document navigation (Cmd+Up/Down on macOS)

    /// Moves cursor to start of document.
    pub fn move_to_start(&mut self, _: &MoveToStart, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    /// Moves cursor to end of document.
    pub fn move_to_end(&mut self, _: &MoveToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.value().len(), cx);
    }

    /// Extends selection to start of document.
    pub fn select_to_start(&mut self, _: &SelectToStart, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(0, cx);
    }

    /// Extends selection to end of document.
    pub fn select_to_end(&mut self, _: &SelectToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.value().len(), cx);
    }

    // Word deletion (Option+Backspace/Delete on macOS)

    /// Deletes from cursor to start of previous word.
    pub fn delete_to_previous_word_start(
        &mut self,
        _: &DeleteToPreviousWordStart,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let offset = self.cursor_offset();
            let prev = self.previous_boundary(offset);
            let target = self.word_start(prev);
            self.select_to(target, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    /// Deletes from cursor to end of next word.
    pub fn delete_to_next_word_end(
        &mut self,
        _: &DeleteToNextWordEnd,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let offset = self.cursor_offset();
            let target = self.word_end(offset);
            self.select_to(target, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    // Line deletion (Cmd+Backspace/Delete on macOS)

    /// Deletes from cursor to start of current line.
    pub fn delete_to_beginning_of_line(
        &mut self,
        _: &DeleteToBeginningOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let (line, _) = self.offset_to_line_col(self.cursor_offset());
            let target = self.line_start_offset(line);
            self.select_to(target, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    /// Deletes from cursor to end of current line.
    pub fn delete_to_end_of_line(
        &mut self,
        _: &DeleteToEndOfLine,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_range.is_empty() {
            let (line, _) = self.offset_to_line_col(self.cursor_offset());
            let target = self.line_end_offset(line);
            self.select_to(target, cx);
        }
        self.replace_text_in_range(None, "", window, cx);
    }

    /// Deletes selection, or one grapheme before cursor if no selection.
    pub fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }

        self.replace_text_in_range(None, "", window, cx)
    }

    /// Deletes selection, or one grapheme after cursor if no selection.
    pub fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }

        self.replace_text_in_range(None, "", window, cx)
    }

    /// Handles scroll wheel events: vertical scroll in wrapped mode, horizontal in non-wrapped.
    pub fn on_scroll_wheel(
        &mut self,
        event: &ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let line_height = self.line_height.unwrap_or(px(16.0));
        let delta = event.delta.pixel_delta(line_height);

        if self.is_wrapped {
            // Wrapped mode: handle vertical scrolling
            let total_visual_lines = self.precomputed_visual_lines.len().max(1);
            let has_vertical_scroll = total_visual_lines > self.line_clamp;

            if has_vertical_scroll && delta.y.abs() > px(0.01) {
                let max_scroll = line_height * (total_visual_lines - self.line_clamp) as f32;
                let new_offset = (self.vertical_scroll_offset - delta.y)
                    .max(Pixels::ZERO)
                    .min(max_scroll);

                if new_offset != self.vertical_scroll_offset {
                    self.vertical_scroll_offset = new_offset;
                    self.is_manually_scrolling = true;
                    cx.stop_propagation();
                    cx.notify();
                }
            }
            return;
        }

        // Non-wrapped mode: check for scrollable content
        let has_vertical_scroll = self.line_count() > self.line_clamp;

        let container_width = self.last_bounds.map(|b| b.size.width).unwrap_or(px(100.0));
        let max_scroll = (self.last_text_width - container_width).max(Pixels::ZERO);
        let has_horizontal_scroll = max_scroll > Pixels::ZERO;

        // Stop propagation if we have scrollable content in the scroll direction
        let is_vertical_scroll = delta.y.abs() > delta.x.abs();
        if (is_vertical_scroll && has_vertical_scroll)
            || (!is_vertical_scroll && has_horizontal_scroll)
        {
            cx.stop_propagation();
        }

        // Only handle horizontal scroll
        if delta.x.abs() < px(0.01) {
            return;
        }

        // Apply horizontal scroll (negative delta.x = scroll right, positive = scroll left)
        let new_offset = (self.horizontal_scroll_offset - delta.x)
            .max(Pixels::ZERO)
            .min(max_scroll);

        if new_offset != self.horizontal_scroll_offset {
            self.horizontal_scroll_offset = new_offset;
            self.is_manually_scrolling = true;
            cx.notify();
        }
    }

    /// Reset manual scrolling flag (called when user types or moves cursor)
    pub(crate) fn reset_manual_scroll(&mut self) {
        self.is_manually_scrolling = false;
    }

    /// Opens the system character palette (macOS emoji/symbol picker).
    pub fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    /// Pastes from clipboard. Newlines become spaces in single-line mode.
    pub fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            // Preserve newlines in multiline mode, replace with spaces in single-line mode
            let text = if self.is_multiline {
                text
            } else {
                text.replace("\n", " ").into()
            };
            self.replace_text_in_range(None, &text, window, cx);
        }
    }

    /// Copies selected text to clipboard. No-op if nothing selected.
    pub fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.value()[self.selected_range.clone()].to_string(),
            ));
        }
    }

    /// Cuts selected text to clipboard. No-op if nothing selected.
    pub fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.value()[self.selected_range.clone()].to_string(),
            ));

            self.replace_text_in_range(None, "", window, cx)
        }
    }

    /// Push current state onto the undo stack before making a change.
    fn push_undo(&mut self) {
        // Remove oldest entry if at capacity
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoEntry {
            text: self.value(),
            selected_range: self.selected_range.clone(),
            selection_reversed: self.selection_reversed,
        });
        // Clear redo stack when a new edit is made
        self.redo_stack.clear();
    }

    /// Restores previous state from undo stack. No-op if stack is empty.
    pub fn undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.undo_stack.pop() {
            // Save current state to redo stack
            self.redo_stack.push(UndoEntry {
                text: self.value(),
                selected_range: self.selected_range.clone(),
                selection_reversed: self.selection_reversed,
            });

            // Restore previous state
            self.value = Some(entry.text);
            self.selected_range = entry.selected_range;
            self.selection_reversed = entry.selection_reversed;

            // Clear layout caches and notify
            self.precomputed_visual_lines.clear();
            self.reset_manual_scroll();
            if self.is_wrapped {
                self.scroll_to_cursor_on_next_render = true;
            } else {
                self.ensure_cursor_visible();
            }
            self.reset_cursor_blink(cx);
            cx.notify();
        }
    }

    /// Restores state from redo stack. No-op if stack is empty.
    pub fn redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.redo_stack.pop() {
            // Save current state to undo stack
            self.undo_stack.push(UndoEntry {
                text: self.value(),
                selected_range: self.selected_range.clone(),
                selection_reversed: self.selection_reversed,
            });

            // Restore redo state
            self.value = Some(entry.text);
            self.selected_range = entry.selected_range;
            self.selection_reversed = entry.selection_reversed;

            // Clear layout caches and notify
            self.precomputed_visual_lines.clear();
            self.reset_manual_scroll();
            if self.is_wrapped {
                self.scroll_to_cursor_on_next_render = true;
            } else {
                self.ensure_cursor_visible();
            }
            self.reset_cursor_blink(cx);
            cx.notify();
        }
    }

    fn move_to_inner(&mut self, offset: usize, scroll: bool, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        if scroll {
            // Reset manual scroll so auto-scroll to cursor works
            self.reset_manual_scroll();
            // For wrapped mode, defer scroll until visual lines are recomputed
            // For non-wrapped mode, scroll immediately since line calculation is always correct
            if self.is_wrapped {
                self.scroll_to_cursor_on_next_render = true;
            } else {
                self.ensure_cursor_visible();
            }
        }
        self.reset_cursor_blink(cx);
        cx.notify()
    }

    /// Sets cursor position and clears selection, scrolling to keep cursor visible.
    pub fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.is_select_all = false;
        self.move_to_inner(offset, true, cx)
    }

    /// Sets cursor position without auto-scrolling. Used by `select_all` to avoid scroll jump.
    pub fn move_to_without_scroll(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.skip_auto_scroll_on_next_render = true;
        self.move_to_inner(offset, false, cx)
    }

    /// Returns the active end of the selection (where the cursor is rendered).
    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    /// Converts from UTF-16 code units (used by IME/platform APIs) to UTF-8 byte offsets (used internally).
    pub fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.value().chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    /// Converts from UTF-8 byte offsets (used internally) to UTF-16 code units (used by IME/platform APIs).
    pub fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.value().chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    /// Range version of `offset_to_utf16` for IME interop.
    pub fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    /// Range version of `offset_from_utf16` for IME interop.
    pub fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }
}

impl EntityInputHandler for InputState {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.value()[range].to_string())
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        Some(UTF16Selection {
            range: self.range_to_utf16(&self.selected_range),
            reversed: self.selection_reversed,
        })
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        self.marked_range
            .as_ref()
            .map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Save current state for undo before making changes
        self.push_undo();

        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        // Build the new text
        let before = &self.value()[0..range.start];
        let after = &self.value()[range.end..];
        let raw_new_text = format!("{}{}{}", before, new_text, after);

        // Apply map_text transformation
        let final_text = self.apply_map_text(raw_new_text);

        // Calculate cursor position, clamping to final text length
        let new_cursor = (range.start + new_text.len()).min(final_text.len());

        self.value = Some(final_text.into());
        self.selected_range = new_cursor..new_cursor;
        self.marked_range.take();

        // Clear precomputed visual lines so they get recomputed with new text
        self.precomputed_visual_lines.clear();

        // Reset manual scroll so auto-scroll to cursor works
        self.reset_manual_scroll();

        // For wrapped mode, defer scroll until visual lines are recomputed
        // For non-wrapped mode, scroll immediately since line calculation is always correct
        if self.is_wrapped {
            self.scroll_to_cursor_on_next_render = true;
        } else {
            self.ensure_cursor_visible();
        }
        self.reset_cursor_blink(cx);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Save current state for undo before making changes
        self.push_undo();

        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        // Build the new text
        let before = &self.value()[0..range.start];
        let after = &self.value()[range.end..];
        let raw_new_text = format!("{}{}{}", before, new_text, after);

        // Apply map_text transformation
        let final_text = self.apply_map_text(raw_new_text);

        self.value = Some(final_text.into());

        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        } else {
            self.marked_range = None;
        }

        self.selected_range = new_selected_range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .map(|new_range| new_range.start + range.start..new_range.end + range.end)
            .unwrap_or_else(|| range.start + new_text.len()..range.start + new_text.len());

        // Clear precomputed visual lines so they get recomputed with new text
        self.precomputed_visual_lines.clear();

        self.reset_cursor_blink(cx);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let range = self.range_from_utf16(&range_utf16);

        // Path 1: Single-line mode (last_layout set by TextElement)
        if let Some(last_layout) = self.last_layout.as_ref() {
            let scroll = self.horizontal_scroll_offset;
            return Some(Bounds::from_corners(
                point(
                    bounds.left() + last_layout.x_for_index(range.start) - scroll,
                    bounds.top(),
                ),
                point(
                    bounds.left() + last_layout.x_for_index(range.end) - scroll,
                    bounds.bottom(),
                ),
            ));
        }

        // Path 2: Multiline (wrapped or non-wrapped) — use visible_lines_info
        // populated during paint by LineElement / WrappedLineElement
        for info in &self.visible_lines_info {
            let (line_start, line_end) = if self.is_wrapped {
                let vl = self.precomputed_visual_lines.get(info.line_index)?;
                (vl.start_offset, vl.end_offset)
            } else {
                (
                    self.line_start_offset(info.line_index),
                    self.line_end_offset(info.line_index),
                )
            };

            if range.start >= line_start && range.start <= line_end {
                let local_start = range.start - line_start;
                let local_end = (range.end - line_start).min(line_end - line_start);
                let scroll = if self.is_wrapped {
                    Pixels::ZERO
                } else {
                    self.horizontal_scroll_offset
                };

                return Some(Bounds::from_corners(
                    point(
                        info.bounds.left() + info.shaped_line.x_for_index(local_start) - scroll,
                        info.bounds.top(),
                    ),
                    point(
                        info.bounds.left() + info.shaped_line.x_for_index(local_end) - scroll,
                        info.bounds.bottom(),
                    ),
                ));
            }
        }

        // Fallback: cursor line not visible (scrolled off-screen) — return
        // bounds at top of container so the menu appears near the input.
        let line_h = self.line_height.unwrap_or(px(20.));
        Some(Bounds::from_corners(
            point(bounds.left(), bounds.top()),
            point(bounds.left(), bounds.top() + line_h),
        ))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let value = self.value();

        // If the value is zero then we can just return zero.
        // Also fixes issue where the assert would fail due to
        // `last_layout.text` being equal to the placeholder text.
        if value.is_empty() {
            Some(0)
        } else {
            let line_point = self.last_bounds?.localize(&point)?;
            let last_layout = self.last_layout.as_ref()?;

            assert_eq!(last_layout.text, value);

            let utf8_index = last_layout.index_for_x(point.x - line_point.x)?;

            Some(self.offset_to_utf16(utf8_index))
        }
    }
}

impl Render for InputState {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl Focusable for InputState {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TextNavigation for InputState {
    fn value(&self) -> SharedString {
        self.value
            .clone()
            .unwrap_or_else(|| SharedString::new_static(""))
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::is_word_char;

    use super::*;

    /// Test helper that implements TextNavigation for testing navigation methods
    /// without requiring a full InputState (which needs FocusHandle/Entity/App context).
    struct TestHelper {
        value: SharedString,
    }

    impl TestHelper {
        fn new(s: &str) -> Self {
            Self {
                value: SharedString::from(s.to_string()),
            }
        }
    }

    impl TextNavigation for TestHelper {
        fn value(&self) -> SharedString {
            self.value.clone()
        }
    }

    #[test]
    fn test_line_count_empty() {
        let state = TestHelper::new("");
        assert_eq!(state.line_count(), 1);
    }

    #[test]
    fn test_line_count_single_line() {
        let state = TestHelper::new("hello world");
        assert_eq!(state.line_count(), 1);
    }

    #[test]
    fn test_line_count_multiple_lines() {
        let state = TestHelper::new("line1\nline2\nline3");
        assert_eq!(state.line_count(), 3);
    }

    #[test]
    fn test_line_count_trailing_newline() {
        let state = TestHelper::new("line1\n");
        assert_eq!(state.line_count(), 2);
    }

    #[test]
    fn test_line_start_offset() {
        let state = TestHelper::new("abc\ndefgh\nij");
        // "abc\ndefgh\nij"
        //  0123 45678 9 10 11
        assert_eq!(state.line_start_offset(0), 0);
        assert_eq!(state.line_start_offset(1), 4); // after "abc\n"
        assert_eq!(state.line_start_offset(2), 10); // after "abc\ndefgh\n"
    }

    #[test]
    fn test_line_end_offset() {
        let state = TestHelper::new("abc\ndefgh\nij");
        assert_eq!(state.line_end_offset(0), 3); // "abc"
        assert_eq!(state.line_end_offset(1), 9); // "defgh"
        assert_eq!(state.line_end_offset(2), 12); // "ij"
    }

    #[test]
    fn test_offset_to_line_col() {
        let state = TestHelper::new("abc\ndefgh\nij");
        assert_eq!(state.offset_to_line_col(0), (0, 0)); // start of line 0
        assert_eq!(state.offset_to_line_col(2), (0, 2)); // 'c' in line 0
        assert_eq!(state.offset_to_line_col(4), (1, 0)); // start of line 1
        assert_eq!(state.offset_to_line_col(7), (1, 3)); // 'g' in line 1
        assert_eq!(state.offset_to_line_col(10), (2, 0)); // start of line 2
        assert_eq!(state.offset_to_line_col(11), (2, 1)); // 'j' in line 2
    }

    #[test]
    fn test_line_col_to_offset() {
        let state = TestHelper::new("abc\ndefgh\nij");
        assert_eq!(state.line_col_to_offset(0, 0), 0);
        assert_eq!(state.line_col_to_offset(0, 2), 2);
        assert_eq!(state.line_col_to_offset(1, 0), 4);
        assert_eq!(state.line_col_to_offset(1, 3), 7);
        assert_eq!(state.line_col_to_offset(2, 0), 10);
        assert_eq!(state.line_col_to_offset(2, 1), 11);
    }

    #[test]
    fn test_line_col_to_offset_clamps_column() {
        let state = TestHelper::new("abc\nde");
        // Column 100 on line 0 (len 3) should clamp to 3
        assert_eq!(state.line_col_to_offset(0, 100), 3);
        // Column 100 on line 1 (len 2) should clamp to 6
        assert_eq!(state.line_col_to_offset(1, 100), 6);
    }

    #[test]
    fn test_visual_line_info_struct() {
        let info = VisualLineInfo {
            start_offset: 0,
            end_offset: 10,
            wrapped_line_index: 0,
            visual_index_in_wrapped: 0,
        };
        assert_eq!(info.start_offset, 0);
        assert_eq!(info.end_offset, 10);
        assert_eq!(info.wrapped_line_index, 0);
        assert_eq!(info.visual_index_in_wrapped, 0);
    }

    // Word boundary tests

    #[test]
    fn test_is_word_char() {
        assert!(is_word_char('a'));
        assert!(is_word_char('Z'));
        assert!(is_word_char('0'));
        assert!(is_word_char('9'));
        assert!(is_word_char('_'));
        assert!(!is_word_char(' '));
        assert!(!is_word_char('-'));
        assert!(!is_word_char('.'));
        assert!(!is_word_char('!'));
        assert!(!is_word_char('\n'));
    }

    #[test]
    fn test_word_start_simple() {
        let state = TestHelper::new("hello world");
        // "hello world"
        //  01234567890
        assert_eq!(state.word_start(0), 0); // at start
        assert_eq!(state.word_start(3), 0); // middle of "hello"
        assert_eq!(state.word_start(5), 0); // end of "hello"
        assert_eq!(state.word_start(6), 5); // on space
        assert_eq!(state.word_start(7), 6); // start of "world"
        assert_eq!(state.word_start(9), 6); // middle of "world"
    }

    #[test]
    fn test_word_end_simple() {
        let state = TestHelper::new("hello world");
        // "hello world"
        //  01234567890
        assert_eq!(state.word_end(0), 5); // start of "hello"
        assert_eq!(state.word_end(3), 5); // middle of "hello"
        assert_eq!(state.word_end(5), 6); // on space
        assert_eq!(state.word_end(6), 11); // start of "world"
        assert_eq!(state.word_end(11), 11); // at end
    }

    #[test]
    fn test_word_boundaries_with_underscore() {
        let state = TestHelper::new("hello_world");
        // "hello_world"
        //  01234567890
        // Underscore should be part of word
        assert_eq!(state.word_start(6), 0); // after underscore, whole thing is one word
        assert_eq!(state.word_end(0), 11); // whole thing is one word
    }

    #[test]
    fn test_word_boundaries_with_hyphen() {
        let state = TestHelper::new("foo-bar");
        // "foo-bar"
        //  0123456
        // Hyphen is NOT a word char, so "foo" and "bar" are separate words
        assert_eq!(state.word_start(2), 0); // in "foo"
        assert_eq!(state.word_end(0), 3); // "foo" ends at 3
        assert_eq!(state.word_start(4), 3); // on hyphen
        assert_eq!(state.word_end(3), 4); // hyphen ends at 4
        assert_eq!(state.word_start(5), 4); // in "bar"
        assert_eq!(state.word_end(4), 7); // "bar" ends at 7
    }

    #[test]
    fn test_word_boundaries_with_numbers() {
        let state = TestHelper::new("test123");
        // Numbers are word chars
        assert_eq!(state.word_start(5), 0);
        assert_eq!(state.word_end(0), 7);
    }

    #[test]
    fn test_word_boundaries_empty() {
        let state = TestHelper::new("");
        assert_eq!(state.word_start(0), 0);
        assert_eq!(state.word_end(0), 0);
    }

    #[test]
    fn test_word_boundaries_punctuation_only() {
        let state = TestHelper::new("...");
        // Each dot should be its own "word"
        assert_eq!(state.word_start(1), 0);
        assert_eq!(state.word_end(0), 1);
        assert_eq!(state.word_end(1), 2);
    }
}
