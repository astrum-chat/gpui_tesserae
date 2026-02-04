use futures_timer::Delay;
use gpui::{
    App, AppContext as _, Application, Bounds, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use gpui_primitives::selectable_text::{self, SelectableText, SelectableTextState};
use std::time::Duration;

struct ExampleApp {
    wrapped_state: Entity<SelectableTextState>,
    text_index: usize,
}

impl Render for ExampleApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().flex().bg(rgb(0x1e1e2e)).p_4().child(
            div()
                .w_full()
                .flex()
                .flex_col()
                .gap_4()
                .child(
                    div()
                        .flex()
                        .text_color(rgb(0xcdd6f4))
                        .text_size(px(18.))
                        .child("Selectable Text w_auto Example (Streaming)"),
                )
                .child(
                    div()
                        .flex()
                        .text_color(rgb(0x6c7086))
                        .text_size(px(12.))
                        .child("Wrapped mode (w_auto):"),
                )
                .child(
                    div()
                        .id("bob")
                        .w_full()
                        .flex()
                        .flex_col()
                        .items_end()
                        .child(
                            div().flex().flex_row().max_w_full().min_w_auto().child(
                                div()
                                    .id("nigel")
                                    .max_w_full()
                                    .min_w_auto()
                                    .h_auto()
                                    .flex()
                                    .flex_row()
                                    .bg(rgb(0x313244))
                                    .p(px(15.))
                                    .rounded_md()
                                    .child(
                                        SelectableText::new(
                                            "wrapped-text",
                                            self.wrapped_state.clone(),
                                        )
                                        .max_w_full()
                                        .min_w_auto()
                                        .w_auto()
                                        .word_wrap(true)
                                        .debug_wrapping(true)
                                        .text_color(rgb(0xcdd6f4))
                                        .text_size(px(16.))
                                        .line_height(px(24.))
                                        .font_family("Geist"),
                                    ),
                            ),
                        ),
                ),
        )
    }
}

const WRAPPED_TEXT: &str = r#"Here's an essay about cabbages:

Cabbages: A Humble yet Remarkable Vegetable

Cabbages are a versatile and nutritious vegetable that have played a significant role in human nutrition and agriculture for thousands of years. From ancient civilizations to modern kitchens, this leafy green has remained a staple food across cultures worldwide.

The origins of cabbage can be traced back to Europe and the Mediterranean region, where wild varieties first grew along coastal cliffs. Ancient Greeks and Romans cultivated cabbage not only for food but also for its perceived medicinal properties.

Nutritionally, cabbages are exceptional. They are low in calories while being incredibly rich in essential nutrients including vitamin C, vitamin K, and dietary fiber. Red cabbage contains anthocyanins, powerful antioxidants that give it its distinctive purple color."#;

fn main() {
    Application::new().run(|cx: &mut App| {
        selectable_text::init(cx);

        let bounds = Bounds::centered(None, size(px(800.), px(400.)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                let wrapped_state = cx.new(|cx| {
                    let mut state = SelectableTextState::new(cx);
                    state.text(""); // Start with empty text
                    state
                });

                cx.new(|cx| {
                    let app = ExampleApp {
                        wrapped_state: wrapped_state.clone(),
                        text_index: 0,
                    };

                    // Stream text in chunks of 4-8 characters with delay between chunks
                    cx.spawn(async move |this, mut cx| {
                        let chars: Vec<char> = WRAPPED_TEXT.chars().collect();
                        let mut i = 0;
                        while i < chars.len() {
                            // Add 4-8 characters at a time
                            let chunk_size = 4 + (i % 5); // varies between 4-8
                            i = (i + chunk_size).min(chars.len());
                            let text: String = chars[..i].iter().collect();
                            let _ = cx.update(|cx| {
                                wrapped_state.update(cx, |state, _cx| {
                                    state.text(&text);
                                });
                                let _ = this.update(cx, |app: &mut ExampleApp, cx| {
                                    app.text_index = i;
                                    cx.notify();
                                });
                            });
                            // Delay between chunks
                            Delay::new(Duration::from_millis(100)).await;
                        }
                    })
                    .detach();

                    app
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
