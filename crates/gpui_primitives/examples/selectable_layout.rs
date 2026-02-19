use gpui::{
    App, AppContext as _, Application, Bounds, Corners, Entity, Font, FontWeight, Hsla,
    IntoElement, ParentElement, Pixels, Render, SharedString, Styled, TextRun, Window,
    WindowBounds, WindowOptions, div, px, rgb, size,
};
use gpui_primitives::selectable_layout::{
    self, InlineStyles, InlinedChild, SelectableLayout, SelectableLayoutState,
};

const SELECTION_COLOR: Hsla = Hsla {
    h: 0.72,
    s: 0.8,
    l: 0.65,
    a: 0.3,
};

fn example_font() -> Font {
    Font {
        family: "Geist".into(),
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// Example InlinedChild children
// ---------------------------------------------------------------------------

/// Plain text span — text with a background color (rendered as an Overlay decoration).
struct TextSpan {
    text: String,
    bg_color: Hsla,
    font: Font,
}

impl TextSpan {
    fn new(text: impl Into<String>, bg_color: Hsla) -> Self {
        Self {
            text: text.into(),
            bg_color,
            font: example_font(),
        }
    }

    fn bold(mut self) -> Self {
        self.font.weight = FontWeight::BOLD;
        self
    }
}

impl InlinedChild for TextSpan {
    fn copy_text(&self) -> SharedString {
        SharedString::from(self.text.clone())
    }

    fn text_run(&self, len: usize) -> TextRun {
        TextRun {
            len,
            font: self.font.clone(),
            color: rgb(0xffffff).into(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }

    fn decoration(&self) -> Option<InlineStyles> {
        Some(
            InlineStyles::new()
                .bg(self.bg_color)
                .corner_radius(Corners::all(px(6.)))
                .corner_smoothing(1.)
                .display(selectable_layout::DecorationDisplay::Overlay),
        )
    }
}

/// Decorated text span — text with a colored background (chip-like appearance).
struct Chip {
    text: String,
    text_color: Hsla,
    bg_color: Hsla,
    font: Font,
}

impl Chip {
    fn new(text: impl Into<String>, bg_color: u32) -> Self {
        Self {
            text: text.into(),
            text_color: rgb(0xffffff).into(),
            bg_color: rgb(bg_color).into(),
            font: example_font(),
        }
    }
}

impl InlinedChild for Chip {
    fn copy_text(&self) -> SharedString {
        SharedString::from(self.text.clone())
    }

    fn text_run(&self, len: usize) -> TextRun {
        TextRun {
            len,
            font: self.font.clone(),
            color: self.text_color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }

    fn decoration(&self) -> Option<InlineStyles> {
        Some(
            InlineStyles::new()
                .bg(self.bg_color)
                .corner_radius(Corners::all(px(4.)))
                .padding_x(px(4.))
                .padding_y(px(2.)),
        )
    }
}

/// Text span with a custom font size.
struct SizedText {
    text: String,
    size: Pixels,
    color: Hsla,
    font: Font,
}

impl SizedText {
    fn new(text: impl Into<String>, size: Pixels, color: Hsla) -> Self {
        Self {
            text: text.into(),
            size,
            color,
            font: example_font(),
        }
    }
}

impl InlinedChild for SizedText {
    fn copy_text(&self) -> SharedString {
        SharedString::from(self.text.clone())
    }

    fn text_run(&self, len: usize) -> TextRun {
        TextRun {
            len,
            font: self.font.clone(),
            color: self.color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }

    fn font_size(&self) -> Option<Pixels> {
        Some(self.size)
    }
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

struct ExampleApp {
    plain_text: Entity<SelectableLayoutState>,
    decorated: Entity<SelectableLayoutState>,
    mixed: Entity<SelectableLayoutState>,
    rounded_selection: Entity<SelectableLayoutState>,
    mixed_sizes: Entity<SelectableLayoutState>,
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
        .w_full()
        .flex_shrink_0()
}

impl Render for ExampleApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let font = example_font();
        let font_size = px(14.);
        let line_height = px(22.);
        let text_color: Hsla = rgb(0xcdd6f4).into();

        div().size_full().flex().bg(rgb(0x1e1e2e)).child(
            div()
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
                        .child("SelectableLayout Examples"),
                )
                // 1. Plain text with wrapping
                .child(label(
                    "Plain text wrapping (click and drag to select, Cmd+C to copy)",
                ))
                .child(
                    container().child(
                        SelectableLayout::new(
                            "plain",
                            self.plain_text.clone(),
                            font.clone(),
                            font_size,
                            line_height,
                            text_color,
                        )
                        .selection_color(SELECTION_COLOR)
                        .child(TextSpan::new(
                            "The quick brown fox jumps over the lazy dog. ",
                            rgb(0x45475a).into(),
                        ))
                        .child(TextSpan::new(
                            "This text wraps at word boundaries ",
                            rgb(0xe06c75).into(),
                        ))
                        .child(TextSpan::new(
                            "and supports character-level selection across children.",
                            rgb(0x61afef).into(),
                        )),
                    ),
                )
                // 2. Decorated chips
                .child(label("Decorated text (chip backgrounds)"))
                .child(
                    container().child(
                        SelectableLayout::new(
                            "decorated",
                            self.decorated.clone(),
                            font.clone(),
                            font_size,
                            line_height,
                            text_color,
                        )
                        .selection_color(SELECTION_COLOR)
                        .child(Chip::new("Rust ", 0xe06c75))
                        .child(Chip::new("GPUI ", 0x61afef))
                        .child(Chip::new("Taffy ", 0x98c379))
                        .child(Chip::new("Zed ", 0xc678dd))
                        .child(Chip::new("Layout ", 0xe5c07b))
                        .child(Chip::new("Inline ", 0x56b6c2))
                        .child(Chip::new("Flow ", 0xd19a66))
                        .child(Chip::new("Wrap", 0xbe5046)),
                    ),
                )
                // 3. Mixed plain and decorated
                .child(label("Mixed plain text and chips"))
                .child(
                    container().child(
                        SelectableLayout::new(
                            "mixed",
                            self.mixed.clone(),
                            font.clone(),
                            font_size,
                            line_height,
                            text_color,
                        )
                        .selection_color(SELECTION_COLOR)
                        .child("Built with ")
                        .child(Chip::new("Rust", 0xe06c75))
                        .child(" and ")
                        .child(Chip::new("GPUI", 0x61afef))
                        .child(" for high-performance text rendering with inline flow and ")
                        .child(TextSpan::new("character-level ", rgb(0x45475a).into()).bold())
                        .child("selection support."),
                    ),
                )
                // 4. Rounded selection
                .child(label("Rounded selection corners"))
                .child(
                    container().child(
                        SelectableLayout::new(
                            "rounded",
                            self.rounded_selection.clone(),
                            font.clone(),
                            font_size,
                            line_height,
                            text_color,
                        )
                        .selection_color(SELECTION_COLOR)
                        .selection_rounded(px(4.))
                        .child(TextSpan::new(
                            "Selection with rounded corners wraps across multiple lines and ",
                            rgb(0x45475a).into(),
                        ))
                        .child(TextSpan::new(
                            "maintains smooth corners ",
                            rgb(0x98c379).into(),
                        ))
                        .child(TextSpan::new("at line transitions.", rgb(0x45475a).into())),
                    ),
                )
                // 5. Mixed font sizes
                .child(label("Mixed font sizes"))
                .child(
                    container().child(
                        SelectableLayout::new(
                            "sizes",
                            self.mixed_sizes.clone(),
                            font.clone(),
                            font_size,
                            line_height,
                            text_color,
                        )
                        .selection_color(SELECTION_COLOR)
                        .child(SizedText::new("Large ", px(22.), rgb(0xe06c75).into()))
                        .child(SizedText::new("Normal ", px(14.), rgb(0xcdd6f4).into()))
                        .child(SizedText::new("Small ", px(10.), rgb(0x98c379).into()))
                        .child(SizedText::new("Tiny ", px(8.), rgb(0xe5c07b).into()))
                        .child(SizedText::new(
                            "text with different sizes wraps and selects correctly.",
                            px(14.),
                            rgb(0xcdd6f4).into(),
                        )),
                    ),
                ),
        )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        selectable_layout::init(cx);

        let bounds = Bounds::centered(None, size(px(550.), px(600.)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_window, cx| {
                let plain_text = cx.new(|cx| SelectableLayoutState::new(cx));
                let decorated = cx.new(|cx| SelectableLayoutState::new(cx));
                let mixed = cx.new(|cx| SelectableLayoutState::new(cx));
                let rounded_selection = cx.new(|cx| SelectableLayoutState::new(cx));
                let mixed_sizes = cx.new(|cx| SelectableLayoutState::new(cx));

                cx.new(|_cx| ExampleApp {
                    plain_text,
                    decorated,
                    mixed,
                    rounded_selection,
                    mixed_sizes,
                })
            },
        )
        .unwrap();

        cx.activate(true);
    });
}
