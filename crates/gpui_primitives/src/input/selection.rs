use gpui::{Context, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point, ScrollStrategy};

use super::state::InputState;
use super::text_navigation::TextNavigation;

impl InputState {
    /// Converts a mouse position to a text offset for single-line inputs.
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
            // Ensure cursor remains visible when selecting
            self.reset_manual_scroll();
            if self.is_wrapped {
                self.scroll_to_cursor_on_next_render = true;
            } else {
                self.ensure_cursor_visible();
            }
        }

        self.reset_cursor_blink(cx);
        cx.notify()
    }

    /// Extends the selection to the given offset, scrolling to keep the cursor visible.
    pub fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.select_to_inner(offset, true, cx)
    }

    /// Extends the selection to the given offset without scrolling.
    pub fn select_to_without_scroll(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.skip_auto_scroll_on_next_render = true;
        self.select_to_inner(offset, false, cx)
    }

    /// Selects the word at the given offset (used for double-click selection).
    pub fn select_word_at(&mut self, offset: usize, cx: &mut Context<Self>) {
        let start = self.word_start(offset);
        let end = self.word_end(start);
        self.selected_range = start..end;
        self.selection_reversed = false;
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

    /// Converts a mouse position to a text offset for multiline inputs, handling wrapped and non-wrapped modes.
    pub fn index_for_multiline_position(
        &self,
        position: Point<Pixels>,
        line_height: Pixels,
    ) -> usize {
        let value = self.value();
        if value.is_empty() {
            return 0;
        }

        if !self.visible_lines_info.is_empty() {
            for info in &self.visible_lines_info {
                if info.bounds.contains(&position) {
                    let local_x = if self.is_wrapped {
                        position.x - info.bounds.left()
                    } else {
                        position.x - info.bounds.left() + self.horizontal_scroll_offset
                    };
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

            if let Some(first) = self.visible_lines_info.first() {
                if position.y < first.bounds.top() {
                    let local_x = if self.is_wrapped {
                        position.x - first.bounds.left()
                    } else {
                        position.x - first.bounds.left() + self.horizontal_scroll_offset
                    };
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

            if let Some(last) = self.visible_lines_info.last() {
                if position.y >= last.bounds.bottom() {
                    let local_x = if self.is_wrapped {
                        position.x - last.bounds.left()
                    } else {
                        position.x - last.bounds.left() + self.horizontal_scroll_offset
                    };
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

        let Some(bounds) = self.last_bounds.as_ref() else {
            return 0;
        };

        let relative_y = position.y - bounds.top();
        let visible_line_index = if relative_y < gpui::px(0.) {
            0
        } else {
            (relative_y / line_height).floor() as usize
        };

        let line_index = visible_line_index;

        if self.is_wrapped {
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

    /// Handles mouse down: starts selection, supports click/double-click/triple-click and shift-extend.
    pub fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut gpui::Window,
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

        if event.click_count >= 3 {
            // Select all without scrolling
            self.move_to_without_scroll(0, cx);
            self.select_to_without_scroll(self.value().len(), cx);
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
        _: &MouseUpEvent,
        _window: &mut gpui::Window,
        _: &mut Context<Self>,
    ) {
        self.is_selecting = false;
    }

    /// Handles mouse move: extends selection while dragging.
    pub fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut gpui::Window,
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
}
