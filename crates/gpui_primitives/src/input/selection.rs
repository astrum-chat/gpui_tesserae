use gpui::{
    Context, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollStrategy, px,
};

use crate::input::state::InputState;
use crate::utils::TextNavigation;

#[allow(missing_docs)]
impl InputState {
    pub fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.value().is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };

        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.value().len();
        }
        if position.x < bounds.left() {
            return 0;
        }
        if position.x > bounds.right() {
            return self.value().len();
        }

        let x_in_text = position.x - bounds.left() + self.horizontal_scroll_offset;
        line.closest_index_for_x(x_in_text)
    }

    fn position_to_index(&self, position: Point<Pixels>) -> usize {
        if self.is_multiline {
            if let Some(line_height) = self.line_height {
                self.index_for_multiline_position(position, line_height)
            } else {
                self.index_for_mouse_position(position)
            }
        } else {
            self.index_for_mouse_position(position)
        }
    }

    fn select_to_inner(&mut self, offset: usize, scroll: bool, cx: &mut Context<Self>) {
        crate::utils::apply_selection_change(
            &mut self.selected_range,
            &mut self.selection_reversed,
            offset,
        );

        if scroll {
            // Ensure cursor remains visible when selecting
            self.reset_manual_scroll();
            self.scroll_to_cursor_horizontal = true;
            if self.is_wrapped {
                self.scroll_to_cursor_on_next_render = true;
            } else {
                self.ensure_cursor_visible();
            }
        }

        self.reset_cursor_blink(cx);
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
        self.reset_cursor_blink(cx);
        cx.notify()
    }

    /// Extends selection to a position in multiline mode, auto-scrolling when dragging past edges.
    pub fn select_to_multiline(
        &mut self,
        position: Point<Pixels>,
        line_height: Pixels,
        cx: &mut Context<Self>,
    ) {
        let offset = self.index_for_multiline_position(position, line_height);
        // Use select_to_without_scroll to prevent ensure_cursor_visible (vertical)
        // from fighting with our explicit scroll_up/down_one_line calls.
        // But opt in to horizontal scroll so cursor stays visible during drag.
        self.select_to_without_scroll(offset, cx);
        self.scroll_to_cursor_horizontal = true;

        if self.is_selecting {
            if let Some(bounds) = &self.last_bounds {
                if let Some(interval_ms) = crate::utils::auto_scroll_vertical_interval(
                    position.y,
                    bounds.top(),
                    bounds.bottom(),
                ) {
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
                }
            }
        }
    }

    pub(crate) fn scroll_up_one_line(&mut self) {
        if self.is_wrapped {
            let line_height = self.line_height.unwrap_or(px(16.0));
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
            let line_height = self.line_height.unwrap_or(px(16.0));
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

    /// Converts a mouse position to a byte offset in multiline mode.
    pub fn index_for_multiline_position(
        &self,
        position: Point<Pixels>,
        line_height: Pixels,
    ) -> usize {
        if self.value().is_empty() {
            return 0;
        }
        crate::utils::index_for_multiline_position(
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

    /// Handles mouse down: click to place cursor, double-click to select word, triple-click to select line.
    pub fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;

        let index = self.position_to_index(event.position);

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

    pub fn on_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut gpui::Window,
        _: &mut Context<Self>,
    ) {
        self.is_selecting = false;
    }

    pub fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            let index = self.position_to_index(event.position);
            self.select_to(index, cx);
        }
    }
}
