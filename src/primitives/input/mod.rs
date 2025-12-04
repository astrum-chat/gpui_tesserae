use gpui::{
    App, Bounds, CursorStyle, Element, ElementId, ElementInputHandler, Entity, GlobalElementId,
    Hsla, InspectorElementId, InteractiveElement, IntoElement, KeyBinding, LayoutId, MouseButton,
    PaintQuad, ParentElement, Pixels, Refineable, RenderOnce, ShapedLine, SharedString, Style,
    StyleRefinement, Styled, TextRun, UnderlineStyle, Window, div, fill, hsla, point,
    prelude::FluentBuilder, px, relative, rgb, size,
};

mod state;
pub use state::*;

use crate::utils::rgb_a;

#[derive(IntoElement)]
pub struct Input {
    state: Entity<InputState>,
    disabled: bool,
    placeholder: SharedString,
    placeholder_text_color: Option<Hsla>,
    selection_color: Option<Hsla>,
    style: StyleRefinement,
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Input {
    pub fn new(state: Entity<InputState>) -> Self {
        Self {
            state,
            disabled: false,
            placeholder: "Type here...".into(),
            placeholder_text_color: None,
            selection_color: None,
            style: StyleRefinement::default(),
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn placeholder_text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.placeholder_text_color = Some(color.into());
        self
    }

    pub fn selection_color(mut self, color: impl Into<Hsla>) -> Self {
        self.selection_color = Some(color.into());
        self
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn initial_value(self, text: impl Into<SharedString>, cx: &mut App) -> Self {
        self.state.update(cx, |this, _| {
            if this.value.is_some() {
                return;
            };
            this.value = Some(text.into());
        });
        self
    }
}

struct TextElement {
    input: Entity<InputState>,
    placeholder: SharedString,
    text_color: Hsla,
    placeholder_text_color: Hsla,
    highlight_text_color: Hsla,
    line_height: Pixels,
}

struct PrepaintState {
    line: Option<ShapedLine>,
    cursor: Option<PaintQuad>,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = PrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = self.line_height.into();

        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let content = input.value();
        let selected_range = input.selected_range.clone();
        let cursor = input.cursor_offset();
        let style = window.text_style();

        let (display_text, text_color) = if content.is_empty() {
            (self.placeholder.clone(), self.placeholder_text_color)
        } else {
            (content, self.text_color)
        };

        let run = TextRun {
            len: display_text.len(),
            font: style.font(),
            color: text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let runs = if let Some(marked_range) = input.marked_range.as_ref() {
            vec![
                TextRun {
                    len: marked_range.start,
                    ..run.clone()
                },
                TextRun {
                    len: marked_range.end - marked_range.start,
                    underline: Some(UnderlineStyle {
                        color: Some(run.color),
                        thickness: px(1.0),
                        wavy: false,
                    }),
                    ..run.clone()
                },
                TextRun {
                    len: display_text.len() - marked_range.end,
                    ..run
                },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect()
        } else {
            vec![run]
        };

        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window
            .text_system()
            .shape_line(display_text, font_size, &runs, None);

        let cursor_pos = line.x_for_index(cursor);
        let (selection, cursor) = if selected_range.is_empty() {
            let height = bounds.bottom() - bounds.top();
            let adjusted_height = height * 0.8;
            let height_diff = height - adjusted_height;

            (
                None,
                Some(fill(
                    Bounds::new(
                        point(bounds.left() + cursor_pos, bounds.top() + height_diff / 2.),
                        size(px(1.), adjusted_height),
                    ),
                    self.text_color,
                )),
            )
        } else {
            (
                Some(fill(
                    Bounds::from_corners(
                        point(
                            bounds.left() + line.x_for_index(selected_range.start),
                            bounds.top(),
                        ),
                        point(
                            bounds.left() + line.x_for_index(selected_range.end),
                            bounds.bottom(),
                        ),
                    ),
                    self.highlight_text_color,
                )),
                None,
            )
        };

        PrepaintState {
            line: Some(line),
            cursor,
            selection,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();

        window.handle_input(
            &focus_handle,
            ElementInputHandler::new(bounds, self.input.clone()),
            cx,
        );

        if let Some(selection) = prepaint.selection.take() {
            window.paint_quad(selection)
        }

        let line = prepaint.line.take().unwrap();
        line.paint(bounds.origin, self.line_height, window, cx)
            .unwrap();

        if focus_handle.is_focused(window)
            && let Some(cursor) = prepaint.cursor.take()
        {
            window.paint_quad(cursor);
        }

        self.input.update(cx, |input, _cx| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = self.state.read(cx);

        let (text_color, line_height) = match &self.style.text {
            Some(text_style) => (
                text_style.color.unwrap_or_else(|| rgb(0xE8E4FF).into()),
                text_style
                    .line_height
                    .map(|this| {
                        this.to_pixels(
                            text_style
                                .font_size
                                .unwrap_or_else(|| window.text_style().font_size),
                            window.rem_size(),
                        )
                    })
                    .unwrap_or_else(|| window.line_height()),
            ),
            None => (rgb(0xE8E4FF).into(), window.line_height()),
        };

        div()
            .map(|mut this| {
                this.style().refine(&self.style);
                this
            })
            .tab_index(0)
            .key_context("TextInput")
            .when(!self.disabled, |this| this.track_focus(&state.focus_handle))
            .cursor(if self.disabled {
                CursorStyle::OperationNotAllowed
            } else {
                CursorStyle::IBeam
            })
            .on_action(window.listener_for(&self.state, InputState::backspace))
            .on_action(window.listener_for(&self.state, InputState::delete))
            .on_action(window.listener_for(&self.state, InputState::left))
            .on_action(window.listener_for(&self.state, InputState::right))
            .on_action(window.listener_for(&self.state, InputState::select_left))
            .on_action(window.listener_for(&self.state, InputState::select_right))
            .on_action(window.listener_for(&self.state, InputState::select_all))
            .on_action(window.listener_for(&self.state, InputState::home))
            .on_action(window.listener_for(&self.state, InputState::end))
            .on_action(window.listener_for(&self.state, InputState::show_character_palette))
            .on_action(window.listener_for(&self.state, InputState::paste))
            .on_action(window.listener_for(&self.state, InputState::cut))
            .on_action(window.listener_for(&self.state, InputState::copy))
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_down),
            )
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_up),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                window.listener_for(&self.state, InputState::on_mouse_up),
            )
            .on_mouse_move(window.listener_for(&self.state, InputState::on_mouse_move))
            .child(TextElement {
                input: self.state,
                placeholder: self.placeholder,
                text_color,
                placeholder_text_color: self
                    .placeholder_text_color
                    .unwrap_or_else(|| hsla(0., 0., 0., 0.2)),
                highlight_text_color: self
                    .selection_color
                    .unwrap_or_else(|| rgb_a(0x488BFF, 0.3).into()),
                line_height,
            })
    }
}

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, None),
        KeyBinding::new("delete", Delete, None),
        KeyBinding::new("left", Left, None),
        KeyBinding::new("right", Right, None),
        KeyBinding::new("shift-left", SelectLeft, None),
        KeyBinding::new("shift-right", SelectRight, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("home", Home, None),
        KeyBinding::new("end", End, None),
        KeyBinding::new("ctrl-cmd-space", ShowCharacterPalette, None),
    ]);

    cx.on_keyboard_layout_change(move |cx| {
        for window in cx.windows() {
            window
                .update(cx, |this, _, cx| cx.notify(this.entity_id()))
                .ok();
        }
    })
    .detach();
}
