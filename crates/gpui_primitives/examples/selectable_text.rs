use gpui::{
    App, AppContext as _, Application, Bounds, Entity, Hsla, InteractiveElement, IntoElement,
    Overflow, ParentElement, Render, Styled, Window, WindowBounds, WindowOptions, div,
    prelude::FluentBuilder, px, rgb, size,
};
use gpui_primitives::selectable_text::{self, SelectableText, SelectableTextState};

const SELECTION_COLOR: Hsla = Hsla {
    h: 0.72,
    s: 0.8,
    l: 0.65,
    a: 0.3,
};

struct ExampleApp {
    basic: Entity<SelectableTextState>,
    multiline: Entity<SelectableTextState>,
    clamped: Entity<SelectableTextState>,
    wrapped: Entity<SelectableTextState>,
    styled: Entity<SelectableTextState>,
    precise: Entity<SelectableTextState>,
}

fn label(text: &str) -> impl IntoElement {
    div()
        .text_color(rgb(0x6c7086))
        .text_size(px(12.))
        .child(text.to_string())
}

fn container() -> gpui::Div {
    div()
        .bg(rgb(0x313244))
        .p(px(12.))
        .rounded_md()
        .w(px(400.))
        .max_w_full()
        .flex_shrink_0()
}

impl Render for ExampleApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div().size_full().flex().bg(rgb(0x1e1e2e)).child(
            div()
                .id("main")
                .map(|mut this| {
                    this.style().overflow.y = Some(Overflow::Scroll);
                    this
                })
                .w_full()
                .flex()
                .flex_col()
                .p_4()
                .gap_3()
                .text_color(rgb(0xcdd6f4))
                .text_size(px(14.))
                .font_family("Geist")
                // Title
                .child(
                    div()
                        .text_size(px(18.))
                        .pb_2()
                        .child("SelectableText Examples"),
                )
                // 1. Basic single-line
                .child(label("Basic single-line"))
                .child(
                    container().child(
                        SelectableText::new("basic", self.basic.clone())
                            .selection_color(SELECTION_COLOR),
                    ),
                )
                // 2. Multiline
                .child(label("Multiline"))
                .child(
                    container().child(
                        SelectableText::new("multiline", self.multiline.clone())
                            .multiline()
                            .selection_color(SELECTION_COLOR),
                    ),
                )
                // 3. Multiline clamped (scroll after 3 lines)
                .child(label("Multiline clamped (3 lines, scroll for more)"))
                .child(
                    container().child(
                        SelectableText::new("clamped", self.clamped.clone())
                            .multiline_clamp(3)
                            .selection_color(SELECTION_COLOR),
                    ),
                )
                // 4. Multiline wrapped
                .child(label("Multiline wrapped (4 lines visible)"))
                .child(
                    container().child(
                        SelectableText::new("wrapped", self.wrapped.clone())
                            .multiline_clamp(4)
                            .multiline_wrapped()
                            .selection_color(SELECTION_COLOR),
                    ),
                )
                // 5. Selection styling
                .child(label("Custom selection styling (rounded)"))
                .child(
                    container().child(
                        SelectableText::new("styled", self.styled.clone())
                            .multiline()
                            .multiline_wrapped()
                            .selection_color(SELECTION_COLOR)
                            .selection_rounded(px(6.)),
                    ),
                )
                // 6. Precise selection
                .child(label(
                    "Precise selection (highlight stops at last character of each line)",
                ))
                .child(
                    container().child(
                        SelectableText::new("precise", self.precise.clone())
                            .multiline()
                            .multiline_wrapped()
                            .selection_color(SELECTION_COLOR)
                            .selection_rounded(px(4.))
                            .selection_precise(),
                    ),
                ),
        )
    }
}

fn make_state(cx: &mut App, text: impl Into<gpui::SharedString>) -> Entity<SelectableTextState> {
    let text = text.into();
    cx.new(|cx| {
        let mut s = SelectableTextState::new(cx);
        s.text(text);
        s
    })
}

fn main() {
    Application::new().run(|cx: &mut App| {
        selectable_text::init(cx);

        let bounds = Bounds::centered(None, size(px(600.), px(700.)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                let basic = make_state(cx, "Hello, world! Try selecting this text.");
                let multiline = make_state(cx, "Line one\nLine two\nLine three\nLine four");
                let clamped = make_state(cx, "First line\nSecond line\nThird line\nFourth line\nFifth line\nSixth line");
                let wrapped = make_state(cx, "Cabbages are a versatile and nutritious vegetable that have played a significant role in human nutrition and agriculture for thousands of years. From ancient civilizations to modern kitchens, this leafy green has remained a staple food across cultures worldwide.");
                let styled = make_state(cx, "Select this text to see rounded selection highlights.\n\nThe corners use a smooth curve for a polished look.");
                let precise = make_state(cx, "Select across lines here. The highlight stops at the last selected character.\n\nIt does not extend to the container edge like the default behavior.");

                cx.new(|_cx| ExampleApp {
                    basic,
                    multiline,
                    clamped,
                    wrapped,
                    styled,
                    precise,
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
