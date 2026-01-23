use gpui::{
    App, AppContext as _, Bounds, ClipboardItem, Context, Entity, EntityInputHandler, FocusHandle,
    Focusable, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, Render,
    ShapedLine, SharedString, UTF16Selection, Window, actions, div, point,
};
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

use super::CursorBlink;

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
        InsertNewline,
        InsertNewlineShift,
        Quit,
    ]
);

pub struct InputState {
    pub focus_handle: FocusHandle,
    pub value: Option<SharedString>,
    pub selected_range: Range<usize>,
    pub selection_reversed: bool,
    pub marked_range: Option<Range<usize>>,
    pub last_layout: Option<ShapedLine>,
    pub last_bounds: Option<Bounds<Pixels>>,
    pub is_selecting: bool,
    pub cursor_blink: Entity<CursorBlink>,
    was_focused: bool,
}

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

    fn reset_cursor_blink(&self, cx: &mut Context<Self>) {
        self.cursor_blink.update(cx, |blink, cx| {
            blink.reset(cx);
        });
    }

    pub fn value(&self) -> SharedString {
        self.value
            .clone()
            .unwrap_or_else(|| SharedString::new_static(""))
    }

    pub fn clear(&mut self) -> Option<SharedString> {
        self.value.take()
    }

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

    pub fn insert_newline(
        &mut self,
        _: &InsertNewline,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(None, "\n", window, cx);
    }

    pub fn insert_newline_shift(
        &mut self,
        _: &InsertNewlineShift,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.replace_text_in_range(None, "\n", window, cx);
    }

    pub fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.value().len(), cx)
    }

    pub fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    pub fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.value().len(), cx);
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

    pub fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;

        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    pub fn on_mouse_up(&mut self, _: &MouseUpEvent, _window: &mut Window, _: &mut Context<Self>) {
        self.is_selecting = false;
    }

    pub fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    pub fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    pub fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace("\n", " "), window, cx);
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

    pub fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        self.reset_cursor_blink(cx);
        cx.notify()
    }

    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    pub fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.value().is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };

        // Handle positions outside bounds for selection during drag
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.value().len();
        }

        // For horizontal positions outside bounds, select to start/end
        if position.x < bounds.left() {
            return 0;
        }
        if position.x > bounds.right() {
            return self.value().len();
        }

        line.closest_index_for_x(position.x - bounds.left())
    }

    pub fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        self.reset_cursor_blink(cx);
        cx.notify()
    }

    /// Select to a position in multi-line mode, accounting for line height
    pub fn select_to_multiline(
        &mut self,
        position: Point<Pixels>,
        line_height: Pixels,
        cx: &mut Context<Self>,
    ) {
        let offset = self.index_for_multiline_position(position, line_height);
        self.select_to(offset, cx);
    }

    /// Calculate the byte offset for a mouse position in multi-line mode
    pub fn index_for_multiline_position(
        &self,
        position: Point<Pixels>,
        line_height: Pixels,
    ) -> usize {
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };

        let value = self.value();
        if value.is_empty() {
            return 0;
        }

        // Calculate which line the position is on
        let relative_y = position.y - bounds.top();
        let line_index = if relative_y < gpui::px(0.) {
            0
        } else {
            (relative_y / line_height).floor() as usize
        };

        let line_count = self.line_count();
        let clamped_line = line_index.min(line_count.saturating_sub(1));

        // Get the line start and end offsets
        let line_start = self.line_start_offset(clamped_line);
        let line_end = self.line_end_offset(clamped_line);

        // If clicking before/after bounds horizontally, select to line start/end
        if position.x < bounds.left() {
            return line_start;
        }
        if position.x > bounds.right() {
            return line_end;
        }

        // For now, use a simple character-based calculation
        // In the future, we could use the line's ShapedLine for precise positioning
        let line_content = &value[line_start..line_end];
        let relative_x = position.x - bounds.left();

        // Estimate character width (this is approximate; proper implementation would use ShapedLine)
        // For now, we'll estimate based on the line content
        if line_content.is_empty() {
            return line_start;
        }

        // Simple linear interpolation based on position
        let ratio = relative_x / bounds.size.width;
        let estimated_char_index = (ratio * line_content.len() as f32) as usize;

        // Find the grapheme boundary
        let mut actual_offset = line_start;
        for (idx, (byte_idx, _)) in line_content.grapheme_indices(true).enumerate() {
            if idx >= estimated_char_index {
                actual_offset = line_start + byte_idx;
                break;
            }
            actual_offset = line_start + byte_idx;
        }

        actual_offset.min(line_end)
    }

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

    pub fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    pub fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    fn previous_boundary(&self, offset: usize) -> usize {
        self.value()
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    fn next_boundary(&self, offset: usize) -> usize {
        self.value()
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.value().len())
    }

    // Multi-line helper methods

    /// Returns the number of lines in the text
    pub fn line_count(&self) -> usize {
        let value = self.value();
        if value.is_empty() {
            1
        } else {
            value.chars().filter(|&c| c == '\n').count() + 1
        }
    }

    /// Returns an iterator over the lines in the text
    pub fn lines(&self) -> impl Iterator<Item = &str> {
        self.value
            .as_ref()
            .map(|v| v.as_ref())
            .unwrap_or("")
            .split('\n')
    }

    /// Returns the byte offset where a line starts
    pub fn line_start_offset(&self, line: usize) -> usize {
        let value = self.value();
        let mut offset = 0;
        for (i, _) in value.split('\n').enumerate() {
            if i == line {
                return offset;
            }
            offset += value[offset..].find('\n').map(|p| p + 1).unwrap_or(0);
        }
        value.len()
    }

    /// Returns the byte offset where a line ends (before the newline, or at text end)
    pub fn line_end_offset(&self, line: usize) -> usize {
        let start = self.line_start_offset(line);
        let value = self.value();
        value[start..]
            .find('\n')
            .map(|p| start + p)
            .unwrap_or(value.len())
    }

    /// Converts a byte offset to (line, column) where column is byte offset within the line
    pub fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        let value = self.value();
        let mut line = 0;
        let mut line_start = 0;

        for (i, c) in value.char_indices() {
            if i >= offset {
                break;
            }
            if c == '\n' {
                line += 1;
                line_start = i + 1;
            }
        }

        (line, offset.saturating_sub(line_start))
    }

    /// Converts (line, column) to byte offset, clamping column to line length
    pub fn line_col_to_offset(&self, line: usize, col: usize) -> usize {
        let line_start = self.line_start_offset(line);
        let line_end = self.line_end_offset(line);
        let line_len = line_end - line_start;
        line_start + col.min(line_len)
    }

    /// Returns the content of a specific line (without the trailing newline)
    pub fn line_content(&self, line: usize) -> &str {
        let start = self.line_start_offset(line);
        let end = self.line_end_offset(line);
        &self.value.as_ref().map(|v| v.as_ref()).unwrap_or("")[start..end]
    }

    /*pub fn reset(&mut self) {
        self.content = "".into();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.last_layout = None;
        self.last_bounds = None;
        self.is_selecting = false;
    }*/
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
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.value = Some(
            (self.value()[0..range.start].to_owned() + new_text + &self.value()[range.end..])
                .into(),
        );
        self.selected_range = range.start + new_text.len()..range.start + new_text.len();
        self.marked_range.take();

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
        let range = range_utf16
            .as_ref()
            .map(|range_utf16| self.range_from_utf16(range_utf16))
            .or(self.marked_range.clone())
            .unwrap_or(self.selected_range.clone());

        self.value = Some(
            (self.value()[0..range.start].to_owned() + new_text + &self.value()[range.end..])
                .into(),
        );

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
        let last_layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        Some(Bounds::from_corners(
            point(
                bounds.left() + last_layout.x_for_index(range.start),
                bounds.top(),
            ),
            point(
                bounds.left() + last_layout.x_for_index(range.end),
                bounds.bottom(),
            ),
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
