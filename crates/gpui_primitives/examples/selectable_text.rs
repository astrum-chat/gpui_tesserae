use gpui::{
    App, AppContext as _, Application, Bounds, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use gpui_primitives::selectable_text::{self, SelectableText, SelectableTextState};

struct ExampleApp {
    wrapped_state: Entity<SelectableTextState>,
}

impl Render for ExampleApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Debug: print state info
        self.wrapped_state.read(cx).debug_widths();

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
                        .child("Selectable Text w_auto Example"),
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
                        .bg(gpui::red())
                        .flex()
                        .flex_col()
                        .items_end()
                        .child(
                            // Flex row wrapper
                            div()
                                .flex()
                                .bg(gpui::black())
                                .flex_row()
                                .max_w_full()
                                .min_w_auto()
                                .child(
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
                                            .bg(gpui::red())
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

const WRAPPED_TEXT: &str = "You're welcome! Feel free to ask if you have any other questions.";

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
                    state.text(WRAPPED_TEXT);
                    state
                });

                cx.new(|_cx| ExampleApp { wrapped_state })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
