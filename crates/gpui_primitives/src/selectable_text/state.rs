use std::ops::Range;
use std::time::Duration;

use gpui::{
    App, Bounds, ClipboardItem, Context, FocusHandle, Focusable, Font, Hsla, IntoElement, Pixels,
    Point, Render, ScrollStrategy, SharedString, Task, UniformListScrollHandle, Window,
    WrappedLine, div,
};

use crate::utils::{
    TextNavigation, WIDTH_WRAP_BASE_MARGIN, apply_selection_change, auto_scroll_horizontal,
    auto_scroll_vertical_interval, clamp_vertical_scroll, compute_max_visual_line_width,
    ensure_cursor_visible_in_scroll, ensure_cursor_visible_wrapped, index_for_multiline_position,
    shape_and_build_visual_lines,
};

pub use crate::utils::{VisibleLineInfo, VisualLineInfo};

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
#[allow(missing_docs)]
pub struct SelectableTextState {
    pub focus_handle: FocusHandle,
    text: SharedString,
    pub selected_range: Range<usize>,
    pub selection_reversed: bool,
    pub is_selecting: bool,

    pub scroll_handle: UniformListScrollHandle,
    pub(crate) multiline_clamp: Option<usize>,
    pub(crate) is_wrapped: bool,
    pub(crate) line_height: Option<Pixels>,

    pub(crate) cached_wrap_width: Option<Pixels>,
    pub(crate) precomputed_visual_lines: Vec<VisualLineInfo>,
    pub(crate) precomputed_wrapped_lines: Vec<WrappedLine>,
    pub(crate) precomputed_at_width: Option<Pixels>,
    #[allow(dead_code)]
    pub(crate) using_auto_width: bool,
    pub(crate) needs_wrap_recompute: bool,
    pub(crate) scroll_to_cursor_on_next_render: bool,

    pub(crate) visible_lines_info: Vec<VisibleLineInfo>,
    pub(crate) last_bounds: Option<Bounds<Pixels>>,
    pub is_select_all: bool,
    pub(crate) measured_max_line_width: Option<Pixels>,
    pub(crate) max_wrapped_line_width: Option<Pixels>,
    pub(crate) is_constrained: bool,
    was_focused: bool,
    pub(crate) last_font: Option<Font>,
    pub(crate) last_font_size: Option<Pixels>,
    pub(crate) last_text_color: Option<Hsla>,

    pub(crate) horizontal_scroll_offset: Pixels,
    pub(crate) vertical_scroll_offset: Pixels,
    pub(crate) last_text_width: Pixels,
    pub(crate) last_scroll_time: Option<std::time::Instant>,
    pub(crate) scroll_to_cursor_horizontal: bool,
    pub(crate) last_mouse_position: Option<Point<Pixels>>,
    auto_scroll_task: Option<Task<()>>,
}

#[allow(missing_docs)]
impl SelectableTextState {
    pub fn new(cx: &App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            text: SharedString::default(),
            selected_range: 0..0,
            selection_reversed: false,
            is_selecting: false,
            scroll_handle: UniformListScrollHandle::new(),
            multiline_clamp: None,
            is_wrapped: false,
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
            max_wrapped_line_width: None,
            is_constrained: false,
            was_focused: false,
            last_font: None,
            last_font_size: None,
            last_text_color: None,
            horizontal_scroll_offset: Pixels::ZERO,
            vertical_scroll_offset: Pixels::ZERO,
            last_text_width: Pixels::ZERO,
            last_scroll_time: None,
            scroll_to_cursor_horizontal: false,
            last_mouse_position: None,
            auto_scroll_task: None,
        }
    }

    pub fn update_focus_state(&mut self, window: &Window) {
        let is_focused = self.focus_handle.is_focused(window);
        if is_focused != self.was_focused {
            self.was_focused = is_focused;
            if !is_focused && !self.selected_range.is_empty() {
                self.selected_range = 0..0;
                self.is_select_all = false;
                self.scroll_to_cursor_horizontal = false;
            }
        }
    }

    /// Sets the text content, clamping any existing selection to the new text length.
    pub fn text(&mut self, text: impl Into<SharedString>) {
        self.text = text.into();
        let text_len = self.text.len();

        let start = self.selected_range.start.min(text_len);
        let end = self.selected_range.end.min(text_len);
        self.selected_range = start..end;

        if self.selected_range.start > self.selected_range.end {
            self.selected_range = 0..0;
            self.selection_reversed = false;
        }

        self.precomputed_visual_lines.clear();
        self.precomputed_wrapped_lines.clear();
        self.needs_wrap_recompute = true;
        self.measured_max_line_width = None;
        self.max_wrapped_line_width = None;
        self.precomputed_at_width = None;
        self.horizontal_scroll_offset = Pixels::ZERO;
        self.last_text_width = Pixels::ZERO;
    }

    pub fn get_text(&self) -> SharedString {
        self.text.clone()
    }

    pub(crate) fn set_multiline_params(
        &mut self,
        line_height: Pixels,
        multiline_clamp: Option<usize>,
    ) {
        self.line_height = Some(line_height);
        self.multiline_clamp = multiline_clamp;
    }

    pub(crate) fn set_wrap_mode(&mut self, wrapped: bool) {
        if self.is_wrapped != wrapped {
            self.cached_wrap_width = None;
            self.precomputed_visual_lines.clear();
            self.precomputed_wrapped_lines.clear();
            self.precomputed_at_width = None;
            self.needs_wrap_recompute = true;
            self.horizontal_scroll_offset = Pixels::ZERO;
        }
        self.is_wrapped = wrapped;
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
        let text = self.get_text();
        self.precomputed_at_width = Some(width);

        let (wrapped_lines, visual_lines) =
            shape_and_build_visual_lines(&text, width, font_size, font, text_color, window);

        let max_line_width = wrapped_lines
            .iter()
            .map(|line| line.unwrapped_layout.width)
            .fold(Pixels::ZERO, |a, b| if b > a { b } else { a });
        self.measured_max_line_width = Some(max_line_width);

        self.max_wrapped_line_width = Some(compute_max_visual_line_width(
            &visual_lines,
            &wrapped_lines,
            &text,
        ));

        self.precomputed_visual_lines = visual_lines;
        self.precomputed_wrapped_lines = wrapped_lines;

        if self.scroll_to_cursor_on_next_render {
            self.scroll_to_cursor_on_next_render = false;
            self.ensure_cursor_visible();
        }

        self.precomputed_visual_lines.len().max(1)
    }

    pub(crate) fn rewrap_at_width(&mut self, width: Pixels, window: &Window) {
        let Some(font) = self.last_font.clone() else {
            return;
        };
        let Some(font_size) = self.last_font_size else {
            return;
        };
        let Some(text_color) = self.last_text_color else {
            return;
        };

        let wrap_width = width + WIDTH_WRAP_BASE_MARGIN;
        let text = self.get_text();

        let (wrapped_lines, visual_lines) =
            shape_and_build_visual_lines(&text, wrap_width, font_size, font, text_color, window);

        let max_line_width = wrapped_lines
            .iter()
            .map(|line| line.unwrapped_layout.width)
            .fold(Pixels::ZERO, |a, b| if b > a { b } else { a });
        self.measured_max_line_width = Some(max_line_width);

        self.max_wrapped_line_width = Some(compute_max_visual_line_width(
            &visual_lines,
            &wrapped_lines,
            &text,
        ));

        self.precomputed_at_width = Some(wrap_width);
        self.precomputed_visual_lines = visual_lines;
        self.precomputed_wrapped_lines = wrapped_lines;
    }

    pub fn ensure_cursor_visible(&mut self) {
        if self.is_wrapped {
            let line_height = self.line_height.unwrap_or(gpui::px(16.0));
            self.vertical_scroll_offset = ensure_cursor_visible_wrapped(
                self.cursor_offset(),
                &self.precomputed_visual_lines,
                line_height,
                self.multiline_clamp,
                self.vertical_scroll_offset,
            );
        } else {
            ensure_cursor_visible_in_scroll(
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

    pub fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    fn select_to_inner(&mut self, offset: usize, scroll: bool, cx: &mut Context<Self>) {
        apply_selection_change(
            &mut self.selected_range,
            &mut self.selection_reversed,
            offset,
        );

        if scroll {
            self.scroll_to_cursor_horizontal = true;
            if self.is_wrapped {
                self.scroll_to_cursor_on_next_render = true;
            } else {
                self.ensure_cursor_visible();
            }
        }

        cx.notify()
    }

    pub fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.is_select_all = false;
        self.select_to_inner(offset, true, cx)
    }

    pub fn select_to_without_scroll(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.scroll_to_cursor_on_next_render = false;
        self.scroll_to_cursor_horizontal = false;
        self.select_to_inner(offset, false, cx)
    }

    pub fn select_word_at(&mut self, offset: usize, cx: &mut Context<Self>) {
        let start = self.word_start(offset);
        let end = self.word_end(start);
        self.selected_range = start..end;
        self.selection_reversed = false;
        self.scroll_to_cursor_on_next_render = false;
        self.scroll_to_cursor_horizontal = false;
        cx.notify()
    }

    fn move_to_inner(&mut self, offset: usize, scroll: bool, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        if scroll {
            self.scroll_to_cursor_horizontal = true;
            if self.is_wrapped {
                self.scroll_to_cursor_on_next_render = true;
            } else {
                self.ensure_cursor_visible();
            }
        }
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

    pub fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        if !self.selected_range.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(
                self.get_text()[self.selected_range.clone()].to_string(),
            ));
        }
    }

    pub fn select_all(&mut self, _: &SelectAll, _: &mut Window, cx: &mut Context<Self>) {
        self.is_select_all = true;
        self.move_to_without_scroll(0, cx);
        self.select_to_without_scroll(self.get_text().len(), cx)
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

    pub fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    pub fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.get_text().len(), cx);
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
        self.move_to(self.get_text().len(), cx);
    }

    pub fn select_to_start(&mut self, _: &SelectToStart, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(0, cx);
    }

    pub fn select_to_end(&mut self, _: &SelectToEnd, _: &mut Window, cx: &mut Context<Self>) {
        self.select_to(self.get_text().len(), cx);
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

    pub fn index_for_multiline_position(
        &self,
        position: gpui::Point<Pixels>,
        line_height: Pixels,
    ) -> usize {
        if self.get_text().is_empty() {
            return 0;
        }
        index_for_multiline_position(
            position,
            line_height,
            self.is_wrapped,
            self.horizontal_scroll_offset,
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
        self.is_select_all = false;
        let offset = self.index_for_multiline_position(position, line_height);
        self.select_to_without_scroll(offset, cx);
        self.scroll_to_cursor_horizontal = true;

        if self.is_selecting {
            if let Some(bounds) = &self.last_bounds {
                if let Some(interval_ms) =
                    auto_scroll_vertical_interval(position.y, bounds.top(), bounds.bottom())
                {
                    let now = std::time::Instant::now();
                    let should_scroll = self
                        .last_scroll_time
                        .map_or(true, |t| now.duration_since(t).as_millis() > interval_ms);

                    if should_scroll {
                        if position.y < bounds.top() {
                            self.scroll_up_one_line();
                        } else {
                            self.scroll_down_one_line();
                        }
                        self.last_scroll_time = Some(now);
                    }
                    self.start_auto_scroll_timer(cx);
                } else {
                    self.auto_scroll_task = None;
                }
            }
        }
    }

    pub(crate) fn ensure_cursor_visible_horizontal(
        &mut self,
        cursor_x: Pixels,
        container_width: Pixels,
    ) -> Pixels {
        if self.is_wrapped {
            return Pixels::ZERO;
        }

        self.horizontal_scroll_offset = auto_scroll_horizontal(
            self.horizontal_scroll_offset,
            cursor_x,
            container_width,
            &mut self.last_scroll_time,
        );
        self.horizontal_scroll_offset
    }

    pub(crate) fn scroll_up_one_line(&mut self) {
        if self.is_wrapped {
            let line_height = self.line_height.unwrap_or(gpui::px(16.0));
            self.vertical_scroll_offset =
                (self.vertical_scroll_offset - line_height).max(Pixels::ZERO);
        } else if let Some(first) = self.visible_lines_info.first() {
            if first.line_index > 0 {
                self.scroll_handle
                    .scroll_to_item(first.line_index - 1, ScrollStrategy::Top);
            }
        }
    }

    pub(crate) fn scroll_down_one_line(&mut self) {
        if self.is_wrapped {
            let line_height = self.line_height.unwrap_or(gpui::px(16.0));
            self.vertical_scroll_offset = self.vertical_scroll_offset + line_height;
            self.clamp_vertical_scroll();
        } else {
            let line_count = self.line_count();
            if let Some(last) = self.visible_lines_info.last() {
                if last.line_index + 1 < line_count {
                    self.scroll_handle
                        .scroll_to_item(last.line_index + 1, ScrollStrategy::Bottom);
                }
            }
        }
    }

    pub(crate) fn clamp_vertical_scroll(&mut self) {
        let line_height = self.line_height.unwrap_or(gpui::px(16.0));
        self.vertical_scroll_offset = clamp_vertical_scroll(
            self.vertical_scroll_offset,
            line_height,
            self.precomputed_visual_lines.len(),
            self.multiline_clamp,
        );
    }

    fn start_auto_scroll_timer(&mut self, cx: &mut Context<Self>) {
        if self.auto_scroll_task.is_some() {
            return;
        }
        self.auto_scroll_task = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
                let Some(this) = this.upgrade() else {
                    break;
                };
                let should_continue = this.update(cx, |state, cx| {
                    if !state.is_selecting {
                        return false;
                    }
                    if let Some(position) = state.last_mouse_position {
                        if let Some(line_height) = state.line_height {
                            state.select_to_multiline(position, line_height, cx);
                        }
                    }
                    true
                });
                if !should_continue {
                    break;
                }
            }
        }));
    }

    /// Handles mouse down: click to place cursor, double-click to select word, triple-click to select line.
    pub fn on_mouse_down(
        &mut self,
        event: &gpui::MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;
        self.is_select_all = false;

        let index = if let Some(line_height) = self.line_height {
            self.index_for_multiline_position(event.position, line_height)
        } else {
            0
        };

        if event.click_count >= 3 {
            self.is_select_all = true;
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

    pub fn on_mouse_up(
        &mut self,
        _: &gpui::MouseUpEvent,
        _window: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.is_selecting = false;
        self.last_mouse_position = None;
        self.auto_scroll_task = None;
    }

    pub fn on_mouse_move(
        &mut self,
        _event: &gpui::MouseMoveEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
    }

    pub fn on_scroll_wheel(
        &mut self,
        event: &gpui::ScrollWheelEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let line_height = self.line_height.unwrap_or(gpui::px(16.0));
        let delta = event.delta.pixel_delta(line_height);

        // Check if there's vertical scrollable content (more lines than visible)
        let line_count = if self.is_wrapped {
            self.precomputed_visual_lines.len()
        } else {
            self.line_count()
        };
        let has_vertical_scroll = self
            .multiline_clamp
            .map_or(false, |clamp| line_count > clamp);

        if has_vertical_scroll {
            cx.stop_propagation();

            // Vertical scroll for wrapped mode
            if self.is_wrapped && delta.y.abs() > gpui::px(0.01) {
                let clamp = self.multiline_clamp.unwrap();
                let max_scroll = line_height * (line_count - clamp) as f32;
                let new_offset = (self.vertical_scroll_offset - delta.y)
                    .max(Pixels::ZERO)
                    .min(max_scroll);

                if new_offset != self.vertical_scroll_offset {
                    self.vertical_scroll_offset = new_offset;
                    cx.notify();
                }
            }
        }

        // Horizontal scroll: only in non-wrapped mode
        if !self.is_wrapped {
            let container_width = self
                .last_bounds
                .map(|b| b.size.width)
                .unwrap_or(gpui::px(100.0));
            let max_scroll = (self.last_text_width - container_width).max(Pixels::ZERO);
            let has_horizontal_scroll = max_scroll > Pixels::ZERO;

            let is_vertical_scroll = delta.y.abs() > delta.x.abs();
            if !is_vertical_scroll && has_horizontal_scroll {
                cx.stop_propagation();
            }

            if delta.x.abs() > gpui::px(0.01) {
                let new_offset = (self.horizontal_scroll_offset - delta.x)
                    .max(Pixels::ZERO)
                    .min(max_scroll);

                if new_offset != self.horizontal_scroll_offset {
                    self.horizontal_scroll_offset = new_offset;
                    cx.notify();
                }
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
