use gpui::{
    App, AppContext as _, Application, Bounds, Context, Entity, IntoElement, ParentElement, Render,
    Styled, Window, WindowBounds, WindowOptions, div, px, rgb, size,
};
use gpui_primitives::selectable_text::{self, SelectableText, SelectableTextState};

struct ExampleApp {
    state: Entity<SelectableTextState>,
}

impl Render for ExampleApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x1e1e2e))
            .p_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .child(
                        div()
                            .text_color(rgb(0xcdd6f4))
                            .text_size(px(18.))
                            .child("Selectable Text Example"),
                    )
                    .child(
                        div()
                            .p_3()
                            .bg(rgb(0x313244))
                            .rounded_md()
                            .child(
                                SelectableText::new("example-text", self.state.clone())
                                    .line_clamp(10)
                                    .word_wrap(true)
                                    .text_color(rgb(0xcdd6f4))
                                    .text_size(px(14.))
                                    .line_height(px(22.))
                                    .font_family("Berkeley Mono"),
                            ),
                    )
                    .child(
                        div()
                            .text_color(rgb(0x6c7086))
                            .text_size(px(12.))
                            .child("Click and drag to select text. Double-click to select a word. Triple-click to select all. Cmd+C to copy."),
                    ),
            )
    }
}

const SAMPLE_TEXT: &str = r#"The selectable_text primitive provides a read-only text component with full selection support.

Features:
- Word wrapping with configurable line clamp
- Click, drag, double-click (word), and triple-click (line) selection
- Keyboard navigation with arrow keys
- Copy to clipboard with Cmd+C / Ctrl+C
- Shift-click to extend selection

This component is useful for displaying code, logs, or any text content where users need to select and copy portions of the text without being able to edit it.

Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat."#;

fn main() {
    Application::new().run(|cx: &mut App| {
        selectable_text::init(cx);

        let bounds = Bounds::centered(None, size(px(600.), px(400.)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                let state = cx.new(|cx| {
                    let mut s = SelectableTextState::new(cx);
                    s.set_text(SAMPLE_TEXT);
                    s
                });

                cx.new(|_cx| ExampleApp { state })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
