use std::sync::Arc;

use gpui::{
    App, AppContext, Application, Bounds, Context, ElementId, Entity, FocusHandle, KeyBinding,
    Menu, Rgba, SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions, actions, div,
    point, prelude::*, px, size,
};
use gpui_transitions::{BoolLerp, TransitionState};

use gpui_tesserae::{
    ElementIdExt, TesseraeAssets, assets,
    components::{
        Button, Checkbox, Input, Switch,
        select::{Select, SelectItemsMap, SelectState},
    },
    extensions::mouse_handleable::MouseHandleable,
    primitives::{
        input::InputState,
        selectable_text::{SelectableText, SelectableTextState},
    },
    theme::{Theme, ThemeExt},
    views::Root,
};

struct Main {
    focus_handle: FocusHandle,

    checkbox_checked: bool,
    switch_checked: bool,
    select_state: Arc<SelectState<SharedString, SharedString>>,
    selectable_text_state: Entity<SelectableTextState>,
}

actions!(window, [TabNext, TabPrev]);

impl Render for Main {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        gpui_tesserae::init_for_window(window, cx);

        let theme = cx.get_theme();

        div()
            .tab_group()
            .track_focus(&self.focus_handle)
            .size_full()
            .text_size(theme.layout.text.default_font.sizes.body)
            .bg(theme.variants.active(cx).colors.background.primary)
            .flex()
            .flex_col()
            .justify_center()
            .items_center()
            .absolute()
            .gap(px(20.))
            .p(px(20.))
            .child(
                Checkbox::new("checkbox")
                    .checked(self.checkbox_checked)
                    .disabled(self.switch_checked)
                    .on_click(cx.listener(|view, checked, _window, cx| {
                        view.checkbox_checked = *checked;
                        cx.notify();
                    })),
            )
            .child(
                Switch::new("switch")
                    .checked(self.switch_checked)
                    .disabled(self.checkbox_checked)
                    .on_click(cx.listener(|view, checked, _window, cx| {
                        view.switch_checked = *checked;
                        cx.notify();
                    })),
            )
            .child(
                Button::new("button")
                    .w(px(300.))
                    .max_w_full()
                    .text("Button")
                    .disabled(self.checkbox_checked || self.switch_checked),
            )
            .child(
                Select::new("select", self.select_state.clone())
                    .w(px(200.))
                    .disabled(self.checkbox_checked || self.switch_checked),
            )
            .child(
                Input::new(
                    "input",
                    window.use_keyed_state(
                        ElementId::from("input").with_suffix("state"),
                        cx,
                        |_window, cx| InputState::new(cx).initial_value("This is a long text that should wrap when the container is narrower than the text width, demonstrating the wrapping behavior"),
                    ),
                )
                .word_wrap(true)
                .w(px(300.))
                .max_w_full()
                .disabled(self.checkbox_checked || self.switch_checked),
            )
            .child({
                let selection_color = cx
                    .get_theme()
                    .variants
                    .active(cx)
                    .colors
                    .accent
                    .primary
                    .alpha(0.3);

                SelectableText::new("selectable-text", self.selectable_text_state.clone())
                    .w(px(300.))
                    .max_w_full()
                    .selection_color(selection_color)
                    .selection_rounded(px(6.))
                    .selection_rounded_smoothing(1.)
            })
    }
}

fn main() {
    Application::new()
        .with_quit_mode(gpui::QuitMode::LastWindowClosed)
        .with_assets(assets![TesseraeAssets])
        .run(|cx: &mut App| {
            gpui_tesserae::init(cx);

            cx.set_menus(vec![Menu {
                name: "My GPUI App".into(),
                items: vec![],
            }]);

            cx.set_theme(Theme::DEFAULT);

            let bounds = Bounds::centered(None, size(px(620.), px(800.)), cx);

            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(10.), px(10.))),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let items = cx.new(|_cx| SelectItemsMap::<SharedString, SharedString>::new());
                    let selected = cx.new(|_cx| None::<SharedString>);
                    let highlighted = cx.new(|_cx| None::<SharedString>);
                    let menu_visible = cx.new(|_cx| TransitionState::new(BoolLerp::falsey()));
                    let focus_handles = cx.new(|_cx| Vec::new());

                    let select_state = Arc::new(SelectState::new(
                        cx,
                        items,
                        selected,
                        highlighted,
                        menu_visible,
                        focus_handles,
                    ));

                    select_state.push_item(cx, SharedString::from("Apple"));
                    select_state.push_item(cx, SharedString::from("Banana"));
                    select_state.push_item(cx, SharedString::from("Cherry"));
                    select_state.push_item(cx, SharedString::from("Date"));

                    let selectable_text_state = cx.new(|cx| {
                        let mut state = SelectableTextState::new(cx);
                        state.text("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.");
                        state
                    });

                    let main = cx.new(|cx| Main {
                        focus_handle: cx.focus_handle(),
                        checkbox_checked: false,
                        switch_checked: false,
                        select_state,
                        selectable_text_state,
                    });

                    cx.new(|cx| Root::new(main, window, cx))
                },
            )
            .unwrap();

            init_tab_indexing_actions(cx);

            cx.activate(true);
        });
}

fn init_tab_indexing_actions(cx: &mut App) {
    cx.on_action(move |_: &TabNext, cx| {
        cx.defer(move |cx| {
            let Some(window) = cx.active_window() else {
                return;
            };

            let _ = window.update(cx, move |_, window, cx| {
                window.focus_next(cx);
            });
        })
    });

    cx.on_action(move |_: &TabPrev, cx| {
        cx.defer(move |cx| {
            let Some(window) = cx.active_window() else {
                return;
            };

            let _ = window.update(cx, move |_, window, cx| {
                window.focus_prev(cx);
            });
        })
    });

    cx.bind_keys([KeyBinding::new("tab", TabNext, None)]);
    cx.bind_keys([KeyBinding::new("shift-tab", TabPrev, None)]);
}

trait RgbaExt {
    fn alpha(self, alpha: f32) -> Self;
}

impl RgbaExt for Rgba {
    fn alpha(mut self, alpha: f32) -> Self {
        self.a = alpha;
        self
    }
}
