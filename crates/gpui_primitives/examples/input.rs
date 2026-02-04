use gpui::{
    App, AppContext as _, Application, Bounds, Context, ElementId, Entity, InteractiveElement,
    IntoElement, Overflow, ParentElement, Render, SharedString, Styled, Window, WindowBounds,
    WindowOptions, div, prelude::FluentBuilder, px, rgb, size,
};
use gpui_primitives::input::{self, Input, InputState, text_transforms};

fn styled_input(id: impl Into<ElementId>, state: Entity<InputState>) -> Input {
    Input::new(id, state)
        .text_color(rgb(0xcdd6f4))
        .placeholder_text_color(rgb(0x6c7086))
        .text_size(px(14.))
        .pl(px(6.))
        .pr(px(6.))
        .pt(px(0.))
        .pb(px(0.))
        .selection_rounded(px(4.))
}

struct ExampleApp {
    // Single-line inputs
    basic_input: Entity<InputState>,
    placeholder_input: Entity<InputState>,
    disabled_input: Entity<InputState>,
    password_input: Entity<InputState>,
    uppercase_input: Entity<InputState>,

    // Multi-line inputs
    multiline_input: Entity<InputState>,
    wrapped_input: Entity<InputState>,
    chat_input: Entity<InputState>,
    clamped_input: Entity<InputState>,
}

impl ExampleApp {
    fn basic_input(&self) -> impl IntoElement {
        input_row(
            "Basic Input",
            "Simple single-line text input",
            styled_input("basic", self.basic_input.clone()).placeholder("Type something..."),
        )
    }

    fn placeholder_input(&self) -> impl IntoElement {
        input_row(
            "Custom Placeholder",
            "With custom placeholder color",
            styled_input("placeholder", self.placeholder_input.clone())
                .placeholder("Enter your email..."),
        )
    }

    fn disabled_input(&self) -> impl IntoElement {
        input_row(
            "Disabled Input",
            "Cannot be focused or edited",
            styled_input("disabled", self.disabled_input.clone())
                .placeholder("This input is disabled")
                .disabled(true),
        )
    }

    fn password_input(&self) -> impl IntoElement {
        input_row(
            "Password Input",
            "Characters masked with transform_text",
            styled_input("password", self.password_input.clone())
                .placeholder("Enter password...")
                .transform_text(text_transforms::password),
        )
    }

    fn uppercase_input(&self) -> impl IntoElement {
        input_row(
            "Uppercase Transform",
            "Text converted to uppercase via map_text",
            styled_input("uppercase", self.uppercase_input.clone())
                .placeholder("Type to see uppercase...")
                .map_text(|text| SharedString::from(text.to_uppercase())),
        )
    }

    fn multiline_input(&self) -> impl IntoElement {
        input_row(
            "Multiline Input",
            "Unlimited lines with vertical scrolling",
            styled_input("multiline", self.multiline_input.clone())
                .multiline()
                .placeholder("Enter multiple lines of text...")
                .min_h(px(100.)),
        )
    }

    fn clamped_input(&self) -> impl IntoElement {
        input_row(
            "Line Clamped (3 lines)",
            "Shows max 3 lines, scrolls after",
            styled_input("clamped", self.clamped_input.clone())
                .line_clamp(3)
                .placeholder("Limited to 3 visible lines..."),
        )
    }

    fn wrapped_input(&self) -> impl IntoElement {
        input_row(
            "Word Wrapped",
            "Text wraps at container width",
            styled_input("wrapped", self.wrapped_input.clone())
                .multiline()
                .word_wrap(true)
                .placeholder("Long text will wrap to the next line...")
                .min_h(px(100.)),
        )
    }

    fn chat_input(&self) -> impl IntoElement {
        input_row(
            "Chat Input",
            "Enter does nothing, Shift+Enter for newline",
            styled_input("chat", self.chat_input.clone())
                .multiline()
                .word_wrap(true)
                .placeholder("Type a message... (Shift+Enter for newline)")
                .secondary_newline()
                .min_h(px(80.)),
        )
    }
}

impl Render for ExampleApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x1e1e2e))
            .p_4()
            .id("example")
            .map(|mut this| {
                this.style().overflow.y = Some(Overflow::Scroll);
                this
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .max_w(px(600.))
                    // Title
                    .child(
                        div()
                            .text_color(rgb(0xcdd6f4))
                            .text_size(px(24.))
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Input Component Examples"),
                    )
                    // Single-line inputs
                    .child(section_header("Single-Line Inputs"))
                    .child(self.basic_input())
                    .child(self.placeholder_input())
                    .child(self.disabled_input())
                    .child(self.password_input())
                    .child(self.uppercase_input())
                    // Multi-line inputs
                    .child(section_header("Multi-Line Inputs"))
                    .child(self.multiline_input())
                    .child(self.clamped_input())
                    .child(self.wrapped_input())
                    .child(self.chat_input())
                    // Keyboard shortcuts
                    .child(section_header("Keyboard Shortcuts"))
                    .child(shortcuts_help()),
            )
    }
}

fn section_header(title: &str) -> impl IntoElement {
    div()
        .mt_4()
        .mb_2()
        .text_color(rgb(0x89b4fa))
        .text_size(px(16.))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(title.to_string())
}

fn input_row(label: &str, description: &str, input: Input) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_color(rgb(0xcdd6f4))
                .text_size(px(13.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child(label.to_string()),
        )
        .child(
            div()
                .text_color(rgb(0x6c7086))
                .text_size(px(11.))
                .child(description.to_string()),
        )
        .child(
            div()
                .mt_1()
                .p_2()
                .bg(rgb(0x313244))
                .rounded_md()
                .child(input),
        )
}

fn shortcuts_help() -> impl IntoElement {
    div()
        .p_3()
        .bg(rgb(0x313244))
        .rounded_md()
        .text_color(rgb(0xa6adc8))
        .text_size(px(12.))
        .line_height(px(20.))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child("Navigation: Arrow keys, Home/End, Cmd+Left/Right")
                .child("Selection: Shift + navigation keys, Cmd+A (select all)")
                .child("Word navigation: Alt+Left/Right")
                .child("Clipboard: Cmd+C (copy), Cmd+X (cut), Cmd+V (paste)")
                .child("Undo/Redo: Cmd+Z / Cmd+Shift+Z")
                .child("Delete word: Alt+Backspace / Alt+Delete")
                .child("Delete line: Cmd+Backspace / Cmd+Delete")
                .child("Mouse: Click to position, double-click for word, triple-click for line"),
        )
}

fn main() {
    Application::new().run(|cx: &mut App| {
        input::init(cx);

        let bounds = Bounds::centered(None, size(px(700.), px(800.)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                let basic_input = cx.new(|cx| InputState::new(cx));
                let placeholder_input = cx.new(|cx| InputState::new(cx));
                let disabled_input =
                    cx.new(|cx| InputState::new(cx).initial_value("Cannot edit this"));
                let password_input = cx.new(|cx| InputState::new(cx));
                let uppercase_input = cx.new(|cx| InputState::new(cx));
                let multiline_input = cx.new(|cx| InputState::new(cx));
                let wrapped_input = cx.new(|cx| {
                    InputState::new(cx).initial_value(
                        "This is a longer piece of text that will wrap to demonstrate \
                         the word wrapping feature of the input component.",
                    )
                });
                let chat_input = cx.new(|cx| InputState::new(cx));
                let clamped_input = cx.new(|cx| {
                    InputState::new(cx).initial_value("Line 1\nLine 2\nLine 3\nLine 4\nLine 5")
                });

                cx.new(|_cx| ExampleApp {
                    basic_input,
                    placeholder_input,
                    disabled_input,
                    password_input,
                    uppercase_input,
                    multiline_input,
                    wrapped_input,
                    chat_input,
                    clamped_input,
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
