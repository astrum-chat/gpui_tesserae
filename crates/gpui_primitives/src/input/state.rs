use gpui::{
    App, AppContext as _, Bounds, ClipboardItem, Context, Entity, EntityInputHandler, FocusHandle,
    Focusable, Hsla, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    Render, ScrollStrategy, ShapedLine, SharedString, TextRun, UTF16Selection,
    UniformListScrollHandle, Window, WrappedLine, actions, div, point,
};
use std::ops::Range;
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

/// Information about a visual line in wrapped text.
/// Maps visual lines (what the user sees) to byte offsets in the text.
#[derive(Clone, Debug)]
pub struct VisualLineInfo {
    /// Byte offset where this visual line starts in the full text
    pub start_offset: usize,
    /// Byte offset where this visual line ends in the full text
    pub end_offset: usize,
    /// The wrapped line this belongs to (index into WrappedLine vec)
    pub wrapped_line_index: usize,
    /// The visual line index within the wrapped line (for multi-wrap scenarios)
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
    pub(crate) max_lines: usize,
    /// Visible line info for uniform_list mode - populated during paint
    pub(crate) visible_lines_info: Vec<VisibleLineInfo>,
    /// Cached container width for wrapped text calculations
    pub(crate) cached_wrap_width: Option<Pixels>,
    /// Pre-computed visual lines for wrapped uniform_list mode
    pub(crate) precomputed_visual_lines: Vec<VisualLineInfo>,
    /// Pre-computed wrapped lines (the actual WrappedLine objects)
    pub(crate) precomputed_wrapped_lines: Vec<WrappedLine>,
    /// Cached text length to detect when text changes
    pub(crate) cached_text_len: usize,
    /// Flag to scroll cursor into view on next render (for wrapped mode)
    /// This defers scrolling until after visual lines are recomputed
    pub(crate) scroll_to_cursor_on_next_render: bool,
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
            is_multiline: false,
            line_height: None,
            map_text: None,
            is_wrapped: false,
            scroll_handle: UniformListScrollHandle::new(),
            max_lines: 1,
            visible_lines_info: Vec::new(),
            cached_wrap_width: None,
            precomputed_visual_lines: Vec::new(),
            precomputed_wrapped_lines: Vec::new(),
            cached_text_len: 0,
            scroll_to_cursor_on_next_render: false,
        }
    }

    /// Set multiline mode parameters (called during render)
    pub(crate) fn set_multiline_params(
        &mut self,
        is_multiline: bool,
        line_height: Pixels,
        max_lines: usize,
    ) {
        self.is_multiline = is_multiline;
        self.line_height = Some(line_height);
        self.max_lines = max_lines;
    }

    /// Pre-compute visual line info for wrapped text.
    /// Called during render to prepare data for uniform_list.
    /// Returns the number of visual lines.
    pub(crate) fn precompute_wrapped_lines(
        &mut self,
        width: Pixels,
        text_color: Hsla,
        window: &Window,
    ) -> usize {
        let text = self.value();
        let text_len = text.len();

        // Check if we need to recompute
        let width_changed = self.cached_wrap_width != Some(width);
        let text_changed = self.cached_text_len != text_len;

        if !width_changed && !text_changed && !self.precomputed_visual_lines.is_empty() {
            return self.precomputed_visual_lines.len();
        }

        // Update cache
        self.cached_wrap_width = Some(width);
        self.cached_text_len = text_len;

        // Clear previous data
        self.precomputed_visual_lines.clear();
        self.precomputed_wrapped_lines.clear();

        if text.is_empty() {
            // For empty text, create one visual line for placeholder
            self.precomputed_visual_lines.push(VisualLineInfo {
                start_offset: 0,
                end_offset: 0,
                wrapped_line_index: 0,
                visual_index_in_wrapped: 0,
            });
            return 1;
        }

        // Shape text with wrapping
        let style = window.text_style();
        let font_size = style.font_size.to_pixels(window.rem_size());

        let run = TextRun {
            len: text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let wrapped_lines = window
            .text_system()
            .shape_text(text.clone(), font_size, &[run], Some(width), None)
            .unwrap_or_default();

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
                // Note: boundary.glyph_ix is the glyph index, not byte offset.
                // We need to get the actual byte offset from glyph.index
                let mut segment_start = 0;
                for (visual_idx, boundary) in wrap_boundaries.iter().enumerate() {
                    // Get the actual byte offset from the glyph structure
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
    /// Uses uniform_list scroll handle for both wrapped and non-wrapped modes.
    pub fn ensure_cursor_visible(&mut self) {
        let cursor_offset = self.cursor_offset();

        if self.is_wrapped {
            // For wrapped mode, find which visual line the cursor is on
            let visual_line = self
                .precomputed_visual_lines
                .iter()
                .position(|info| {
                    cursor_offset >= info.start_offset && cursor_offset <= info.end_offset
                })
                .unwrap_or(0);

            self.scroll_handle
                .scroll_to_item(visual_line, ScrollStrategy::Center);
        } else {
            // For non-wrapped mode, use logical line
            let cursor_line = self.offset_to_line_col(cursor_offset).0;
            self.scroll_handle
                .scroll_to_item(cursor_line, ScrollStrategy::Center);
        }
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

        let index = if self.is_multiline {
            if let Some(line_height) = self.line_height {
                self.index_for_multiline_position(event.position, line_height)
            } else {
                self.index_for_mouse_position(event.position)
            }
        } else {
            self.index_for_mouse_position(event.position)
        };

        if event.modifiers.shift {
            self.select_to(index, cx);
        } else {
            self.move_to(index, cx)
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
            let index = if self.is_multiline {
                if let Some(line_height) = self.line_height {
                    self.index_for_multiline_position(event.position, line_height)
                } else {
                    self.index_for_mouse_position(event.position)
                }
            } else {
                self.index_for_mouse_position(event.position)
            };
            self.select_to(index, cx);
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
        // For wrapped mode, defer scroll until visual lines are recomputed
        // For non-wrapped mode, scroll immediately since line calculation is always correct
        if self.is_wrapped {
            self.scroll_to_cursor_on_next_render = true;
        } else {
            self.ensure_cursor_visible();
        }
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

    /// Select to a position in multi-line mode, accounting for line height.
    /// Also auto-scrolls when dragging outside the input bounds.
    pub fn select_to_multiline(
        &mut self,
        position: Point<Pixels>,
        line_height: Pixels,
        cx: &mut Context<Self>,
    ) {
        let offset = self.index_for_multiline_position(position, line_height);
        self.select_to(offset, cx);

        // Auto-scroll when dragging outside bounds
        if self.is_selecting {
            if let Some(bounds) = &self.last_bounds {
                if position.y < bounds.top() {
                    // Dragging above - scroll up
                    self.scroll_up_one_line();
                } else if position.y > bounds.bottom() {
                    // Dragging below - scroll down
                    self.scroll_down_one_line();
                }
            }
        }
    }

    /// Scroll up by one line (used for drag-to-select auto-scrolling)
    fn scroll_up_one_line(&self) {
        if let Some(first) = self.visible_lines_info.first() {
            if first.line_index > 0 {
                self.scroll_handle
                    .scroll_to_item(first.line_index - 1, ScrollStrategy::Top);
            }
        }
    }

    /// Scroll down by one line (used for drag-to-select auto-scrolling)
    fn scroll_down_one_line(&self) {
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

    /// Calculate the byte offset for a mouse position in multi-line mode
    pub fn index_for_multiline_position(
        &self,
        position: Point<Pixels>,
        line_height: Pixels,
    ) -> usize {
        let value = self.value();
        if value.is_empty() {
            return 0;
        }

        // Use visible_lines_info for accurate hit testing (works for both wrapped and non-wrapped)
        if !self.visible_lines_info.is_empty() {
            // Check if position is within any visible line
            for info in &self.visible_lines_info {
                if info.bounds.contains(&position) {
                    let local_x = position.x - info.bounds.left();
                    let local_index = info.shaped_line.closest_index_for_x(local_x);

                    // For wrapped mode, line_index is visual line index - look up byte offset
                    if self.is_wrapped {
                        if let Some(visual_info) =
                            self.precomputed_visual_lines.get(info.line_index)
                        {
                            return visual_info.start_offset + local_index;
                        }
                    }
                    // For non-wrapped mode, line_index is actual line index
                    let line_start = self.line_start_offset(info.line_index);
                    return line_start + local_index;
                }
            }

            // Position is outside visible lines - find closest line
            // Check if above visible area
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

            // Check if below visible area
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

            // Position is horizontally outside but vertically within - find the line by Y
            for info in &self.visible_lines_info {
                if position.y >= info.bounds.top() && position.y < info.bounds.bottom() {
                    if self.is_wrapped {
                        if let Some(visual_info) =
                            self.precomputed_visual_lines.get(info.line_index)
                        {
                            if position.x < info.bounds.left() {
                                return visual_info.start_offset;
                            }
                            if position.x > info.bounds.right() {
                                return visual_info.end_offset;
                            }
                        }
                    } else {
                        let line_start = self.line_start_offset(info.line_index);
                        let line_end = self.line_end_offset(info.line_index);
                        if position.x < info.bounds.left() {
                            return line_start;
                        }
                        if position.x > info.bounds.right() {
                            return line_end;
                        }
                    }
                }
            }
        }

        // Fallback to old calculation if visible_lines_info is empty
        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };

        let relative_y = position.y - bounds.top();
        let visible_line_index = if relative_y < gpui::px(0.) {
            0
        } else {
            (relative_y / line_height).floor() as usize
        };

        let scroll_offset = self.scroll_handle.logical_scroll_top_index();
        let line_index = visible_line_index + scroll_offset;

        if self.is_wrapped {
            // For wrapped mode fallback, use precomputed_visual_lines
            let visual_line_count = self.precomputed_visual_lines.len();
            let clamped_visual_line = line_index.min(visual_line_count.saturating_sub(1));
            if let Some(visual_info) = self.precomputed_visual_lines.get(clamped_visual_line) {
                return visual_info.start_offset;
            }
        }

        let line_count = self.line_count();
        let clamped_line = line_index.min(line_count.saturating_sub(1));

        self.line_start_offset(clamped_line)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper that only tests methods that use `value` field.
    /// We avoid creating full InputState since FocusHandle/Entity require App context.
    struct TestValue {
        value: Option<SharedString>,
    }

    impl TestValue {
        fn new(s: &str) -> Self {
            Self {
                value: Some(SharedString::from(s.to_string())),
            }
        }

        fn value(&self) -> SharedString {
            self.value
                .clone()
                .unwrap_or_else(|| SharedString::new_static(""))
        }

        fn line_count(&self) -> usize {
            let value = self.value();
            if value.is_empty() {
                1
            } else {
                value.chars().filter(|&c| c == '\n').count() + 1
            }
        }

        fn line_start_offset(&self, line: usize) -> usize {
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

        fn line_end_offset(&self, line: usize) -> usize {
            let start = self.line_start_offset(line);
            let value = self.value();
            value[start..]
                .find('\n')
                .map(|p| start + p)
                .unwrap_or(value.len())
        }

        fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
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

        fn line_col_to_offset(&self, line: usize, col: usize) -> usize {
            let line_start = self.line_start_offset(line);
            let line_end = self.line_end_offset(line);
            let line_len = line_end - line_start;
            line_start + col.min(line_len)
        }

        fn line_content(&self, line: usize) -> &str {
            let start = self.line_start_offset(line);
            let end = self.line_end_offset(line);
            &self.value.as_ref().map(|v| v.as_ref()).unwrap_or("")[start..end]
        }
    }

    #[test]
    fn test_line_count_empty() {
        let state = TestValue::new("");
        assert_eq!(state.line_count(), 1);
    }

    #[test]
    fn test_line_count_single_line() {
        let state = TestValue::new("hello world");
        assert_eq!(state.line_count(), 1);
    }

    #[test]
    fn test_line_count_multiple_lines() {
        let state = TestValue::new("line1\nline2\nline3");
        assert_eq!(state.line_count(), 3);
    }

    #[test]
    fn test_line_count_trailing_newline() {
        let state = TestValue::new("line1\n");
        assert_eq!(state.line_count(), 2);
    }

    #[test]
    fn test_line_start_offset() {
        let state = TestValue::new("abc\ndefgh\nij");
        // "abc\ndefgh\nij"
        //  0123 45678 9 10 11
        assert_eq!(state.line_start_offset(0), 0);
        assert_eq!(state.line_start_offset(1), 4); // after "abc\n"
        assert_eq!(state.line_start_offset(2), 10); // after "abc\ndefgh\n"
    }

    #[test]
    fn test_line_end_offset() {
        let state = TestValue::new("abc\ndefgh\nij");
        assert_eq!(state.line_end_offset(0), 3); // "abc"
        assert_eq!(state.line_end_offset(1), 9); // "defgh"
        assert_eq!(state.line_end_offset(2), 12); // "ij"
    }

    #[test]
    fn test_offset_to_line_col() {
        let state = TestValue::new("abc\ndefgh\nij");
        assert_eq!(state.offset_to_line_col(0), (0, 0)); // start of line 0
        assert_eq!(state.offset_to_line_col(2), (0, 2)); // 'c' in line 0
        assert_eq!(state.offset_to_line_col(4), (1, 0)); // start of line 1
        assert_eq!(state.offset_to_line_col(7), (1, 3)); // 'g' in line 1
        assert_eq!(state.offset_to_line_col(10), (2, 0)); // start of line 2
        assert_eq!(state.offset_to_line_col(11), (2, 1)); // 'j' in line 2
    }

    #[test]
    fn test_line_col_to_offset() {
        let state = TestValue::new("abc\ndefgh\nij");
        assert_eq!(state.line_col_to_offset(0, 0), 0);
        assert_eq!(state.line_col_to_offset(0, 2), 2);
        assert_eq!(state.line_col_to_offset(1, 0), 4);
        assert_eq!(state.line_col_to_offset(1, 3), 7);
        assert_eq!(state.line_col_to_offset(2, 0), 10);
        assert_eq!(state.line_col_to_offset(2, 1), 11);
    }

    #[test]
    fn test_line_col_to_offset_clamps_column() {
        let state = TestValue::new("abc\nde");
        // Column 100 on line 0 (len 3) should clamp to 3
        assert_eq!(state.line_col_to_offset(0, 100), 3);
        // Column 100 on line 1 (len 2) should clamp to 6
        assert_eq!(state.line_col_to_offset(1, 100), 6);
    }

    #[test]
    fn test_line_content() {
        let state = TestValue::new("abc\ndefgh\nij");
        assert_eq!(state.line_content(0), "abc");
        assert_eq!(state.line_content(1), "defgh");
        assert_eq!(state.line_content(2), "ij");
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
}
