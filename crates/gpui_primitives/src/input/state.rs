use std::ops::Range;
use std::sync::Arc;

use gpui::{
    App, AppContext as _, Bounds, ClipboardItem, Context, Entity, EntityInputHandler, FocusHandle,
    Focusable, Font, Hsla, IntoElement, Pixels, Render, ScrollWheelEvent, ShapedLine, SharedString,
    UTF16Selection, UniformListScrollHandle, Window, WrappedLine, div, point, px,
};

use crate::input::CursorBlink;
use crate::utils::TextNavigation;

pub use crate::utils::{VisibleLineInfo, VisualLineInfo};

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
#[allow(missing_docs)]
pub struct InputState {
    pub focus_handle: FocusHandle,
    pub value: Option<SharedString>,
    /// Byte range of the current selection. Empty range means cursor position only.
    pub selected_range: Range<usize>,
    /// If true, the cursor is at selection start; if false, at selection end.
    pub selection_reversed: bool,
    /// Byte range of IME composition text (marked text), if any.
    pub marked_range: Option<Range<usize>>,
    pub last_layout: Option<ShapedLine>,
    pub last_bounds: Option<Bounds<Pixels>>,
    /// True while the user is dragging to select text.
    pub is_selecting: bool,
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
    /// Maximum visible lines (for scroll calculations). None = unlimited.
    pub(crate) multiline_clamp: Option<usize>,
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
    /// Vertical scroll offset for wrapped mode with multiline_clamp (in pixels)
    pub(crate) vertical_scroll_offset: Pixels,
    /// Last measured text width (for scroll wheel calculations)
    pub(crate) last_text_width: Pixels,
    /// When true, skip auto-scroll to cursor (user is manually scrolling)
    pub(crate) is_manually_scrolling: bool,
    /// When true, auto-scroll horizontally to keep cursor visible on next render.
    /// Opt-in: set by navigation/editing, NOT by double-click/triple-click/select-all.
    pub(crate) scroll_to_cursor_horizontal: bool,
    /// Timestamp of last auto-scroll frame, for delta-time-based scrolling.
    pub(crate) last_scroll_time: Option<std::time::Instant>,
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

#[allow(missing_docs)]
impl InputState {
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
            multiline_clamp: None,
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
            scroll_to_cursor_horizontal: false,
            last_scroll_time: None,
            is_select_all: false,
            last_font: None,
            last_font_size: None,
            last_text_color: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_history: 200,
        }
    }

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

    pub(crate) fn set_multiline_params(
        &mut self,
        is_multiline: bool,
        line_height: Pixels,
        multiline_clamp: Option<usize>,
    ) {
        self.is_multiline = is_multiline;
        self.line_height = Some(line_height);
        self.multiline_clamp = multiline_clamp;
    }

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
            let total_visual_lines = self.precomputed_visual_lines.len().max(1);
            let visible_height = line_height
                * self
                    .multiline_clamp
                    .map_or(1, |c| c.min(total_visual_lines)) as f32;

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
                self.multiline_clamp,
                &self.scroll_handle,
                |offset| self.offset_to_line_col(offset).0,
                || self.line_count(),
            );
        }
    }

    pub(crate) fn clamp_vertical_scroll(&mut self) {
        let line_height = self.line_height.unwrap_or(px(16.0));
        let total_lines = self.precomputed_visual_lines.len().max(1);
        let visible_lines = self.multiline_clamp.map_or(1, |c| c.min(total_lines));
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

        self.horizontal_scroll_offset = crate::utils::auto_scroll_horizontal(
            self.horizontal_scroll_offset,
            cursor_x,
            container_width,
            &mut self.last_scroll_time,
        );
        self.horizontal_scroll_offset
    }

    fn apply_map_text(&self, text: String) -> String {
        if let Some(map_fn) = &self.map_text {
            map_fn(text.into()).to_string()
        } else {
            text
        }
    }

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

    pub fn cursor_visible(&self, cx: &App) -> bool {
        self.cursor_blink.read(cx).visible()
    }

    pub fn start_cursor_blink(&self, cx: &mut Context<Self>) {
        self.cursor_blink.update(cx, |blink, cx| {
            blink.start(cx);
        });
    }

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

    pub fn value(&self) -> SharedString {
        self.value
            .clone()
            .unwrap_or_else(|| SharedString::new_static(""))
    }

    /// Clears the text value and resets selection. Returns the previous value.
    pub fn clear(&mut self) -> Option<SharedString> {
        self.selected_range = 0..0;
        self.value.take()
    }

    /// Sets initial text only if value is currently unset.
    pub fn initial_value(mut self, text: impl Into<SharedString>) -> Self {
        if self.value.is_some() {
            return self;
        };
        self.value = Some(text.into());
        self
    }

    pub fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.previous_boundary(self.cursor_offset()), cx);
        } else {
            self.move_to(self.selected_range.start, cx)
        }
    }

    pub fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.move_to(self.next_boundary(self.selected_range.end), cx);
        } else {
            self.move_to(self.selected_range.end, cx)
        }
    }

    pub fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
    }

    pub fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
    }

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

    pub fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        let (line, col) = self.offset_to_line_col(self.cursor_offset());
        if line > 0 {
            let new_offset = self.line_col_to_offset(line - 1, col);
            self.select_to(new_offset, cx);
        }
    }

    pub fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        let (line, col) = self.offset_to_line_col(self.cursor_offset());
        if line < self.line_count().saturating_sub(1) {
            let new_offset = self.line_col_to_offset(line + 1, col);
            self.select_to(new_offset, cx);
        }
    }

    /// Inserts a newline at the cursor position (primary submit action in multiline mode).
    pub fn insert_newline(&mut self, _: &Submit, window: &mut Window, cx: &mut Context<Self>) {
        self.replace_text_in_range(None, "\n", window, cx);
    }

    /// Inserts a newline at the cursor position (secondary submit action).
    pub fn insert_newline_secondary(
        &mut self,
        _: &SecondarySubmit,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(None, "\n", window, cx);
    }

    pub fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.is_select_all = true;
        self.move_to_without_scroll(0, cx);
        self.select_to_without_scroll(self.value().len(), cx)
    }

    pub fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    pub fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.value().len(), cx);
    }

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

    pub fn move_to_start(&mut self, _: &MoveToStart, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    pub fn move_to_end(&mut self, _: &MoveToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.value().len(), cx);
    }

    pub fn select_to_start(&mut self, _: &SelectToStart, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(0, cx);
    }

    pub fn select_to_end(&mut self, _: &SelectToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.value().len(), cx);
    }

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

    pub fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }

        self.replace_text_in_range(None, "", window, cx)
    }

    pub fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }

        self.replace_text_in_range(None, "", window, cx)
    }

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
            let has_vertical_scroll = self
                .multiline_clamp
                .map_or(false, |clamp| total_visual_lines > clamp);

            if has_vertical_scroll && delta.y.abs() > px(0.01) {
                let clamp = self.multiline_clamp.unwrap(); // safe: has_vertical_scroll implies Some
                let max_scroll = line_height * (total_visual_lines - clamp) as f32;
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
        let has_vertical_scroll = self
            .multiline_clamp
            .map_or(false, |clamp| self.line_count() > clamp);

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

    pub(crate) fn reset_manual_scroll(&mut self) {
        self.is_manually_scrolling = false;
    }

    /// Opens the system character palette (emoji/symbol picker).
    pub fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    /// Newlines become spaces in single-line mode.
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

    pub fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.value()[self.selected_range.clone()].to_string(),
            ));
        }
    }

    pub fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.value()[self.selected_range.clone()].to_string(),
            ));

            self.replace_text_in_range(None, "", window, cx)
        }
    }

    fn push_undo(&mut self) {
        if self.undo_stack.len() >= self.max_history {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(UndoEntry {
            text: self.value(),
            selected_range: self.selected_range.clone(),
            selection_reversed: self.selection_reversed,
        });
        self.redo_stack.clear();
    }

    fn apply_history_entry(&mut self, entry: UndoEntry, cx: &mut Context<Self>) {
        self.value = Some(entry.text);
        self.selected_range = entry.selected_range;
        self.selection_reversed = entry.selection_reversed;
        self.precomputed_visual_lines.clear();
        self.reset_manual_scroll();
        self.scroll_to_cursor_horizontal = true;
        if self.is_wrapped {
            self.scroll_to_cursor_on_next_render = true;
        } else {
            self.ensure_cursor_visible();
        }
        self.reset_cursor_blink(cx);
        cx.notify();
    }

    pub fn undo(&mut self, _: &Undo, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.undo_stack.pop() {
            self.redo_stack.push(UndoEntry {
                text: self.value(),
                selected_range: self.selected_range.clone(),
                selection_reversed: self.selection_reversed,
            });
            self.apply_history_entry(entry, cx);
        }
    }

    pub fn redo(&mut self, _: &Redo, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(entry) = self.redo_stack.pop() {
            self.undo_stack.push(UndoEntry {
                text: self.value(),
                selected_range: self.selected_range.clone(),
                selection_reversed: self.selection_reversed,
            });
            self.apply_history_entry(entry, cx);
        }
    }

    fn move_to_inner(&mut self, offset: usize, scroll: bool, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        if scroll {
            // Reset manual scroll so auto-scroll to cursor works
            self.reset_manual_scroll();
            self.scroll_to_cursor_horizontal = true;
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

    pub fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.is_select_all = false;
        self.move_to_inner(offset, true, cx)
    }

    pub fn move_to_without_scroll(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.scroll_to_cursor_on_next_render = false;
        self.scroll_to_cursor_horizontal = false;
        self.move_to_inner(offset, false, cx)
    }

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
        self.scroll_to_cursor_horizontal = true;

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

        // Path 2: Multiline (wrapped or non-wrapped) - use visible_lines_info
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

        // Fallback: cursor line not visible (scrolled off-screen) - return
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
