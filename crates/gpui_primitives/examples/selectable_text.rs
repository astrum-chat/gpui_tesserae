use gpui::{
    App, AppContext as _, Application, Bounds, Context, Entity, InteractiveElement, IntoElement,
    ParentElement, Render, Styled, Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use gpui_primitives::selectable_text::{self, SelectableText, SelectableTextState};

struct ExampleApp {
    wrapped_state: Entity<SelectableTextState>,
    //non_wrapped_state: Entity<SelectableTextState>,
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
                        .child("Selectable Text w_auto Example"),
                )
                // Wrapped mode with w_auto
                .child(
                    div()
                        .flex()
                        .text_color(rgb(0x6c7086))
                        .text_size(px(12.))
                        .child("Wrapped mode (w_auto):"),
                )
                .child(
                    div()
                        .id("nigel")
                        .w_full()
                        .h_auto()
                        .flex()
                        .items_center()
                        .bg(rgb(0x313244))
                        .rounded_md()
                        .child(div().bg(gpui::blue()).size(px(250.)))
                        .child(
                            div()
                                .id("bob")
                                .flex()
                                .w_full()
                                .min_w_0()
                                .h_auto()
                                .flex()
                                .child(
                                    SelectableText::new("wrapped-text", self.wrapped_state.clone())
                                        .max_w_full()
                                        .w_auto()
                                        .word_wrap(true)
                                        .line_clamp(5)
                                        .bg(gpui::red())
                                        .text_color(rgb(0xcdd6f4))
                                        .text_size(px(16.))
                                        .line_height(px(24.))
                                        .font_family("Geist"),
                                ),
                        ),
                ), // Non-wrapped mode with w_auto
                   /*.child(
                       div()
                           .text_color(rgb(0x6c7086))
                           .text_size(px(12.))
                           .child("Non-wrapped mode (w_auto):"),
                   )
                   .child(
                       div().w_auto().p_3().bg(rgb(0x313244)).rounded_md().child(
                           SelectableText::new("non-wrapped-text", self.non_wrapped_state.clone())
                               .w_auto()
                               .line_clamp(3)
                               .word_wrap(false)
                               .text_color(rgb(0xcdd6f4))
                               .text_size(px(14.))
                               .line_height(px(22.))
                               .bg(gpui::red())
                               .font_family("Berkeley Mono"),
                       ),
                   )
                   .child(
                       div()
                           .text_color(rgb(0x6c7086))
                           .text_size(px(12.))
                           .child("Both boxes should size to their content width."),
                   ),*/
        )
    }
}

// Medium text that fits in wide container but needs wrap when shrunk
const WRAPPED_TEXT: &str = "You're welcome! Feel free to ask if you have any other questions.";

/*const NON_WRAPPED_TEXT: &str = "Line one
Line two is longer
Short";*/

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

                /*let non_wrapped_state = cx.new(|cx| {
                    let mut state = SelectableTextState::new(cx);
                    state.text(NON_WRAPPED_TEXT);
                    state
                });*/

                cx.new(|_cx| ExampleApp {
                    wrapped_state,
                    //non_wrapped_state,
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
