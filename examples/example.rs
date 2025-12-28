use gpui::{
    App, AppContext, Application, Bounds, Context, ElementId, FocusHandle, KeyBinding, Menu,
    TitlebarOptions, Window, WindowBounds, WindowOptions, actions, div, point, prelude::*, px,
    size,
};

use gpui_tesserae::{
    ElementIdExt, TesseraeAssets, assets,
    components::{Button, Checkbox, Input, Switch},
    primitives::input::InputState,
    theme::{Theme, ThemeExt},
};

struct Root {
    focus_handle: FocusHandle,

    checkbox_checked: bool,
    switch_checked: bool,
}

actions!(window, [TabNext, TabPrev]);

impl Render for Root {
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
            .p(px(100.))
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
                Input::new(
                    "input",
                    window.use_keyed_state(
                        ElementId::from("input").with_suffix("state"),
                        cx,
                        |_window, cx| InputState::new(cx),
                    ),
                )
                .disabled(self.checkbox_checked || self.switch_checked)
                .map(|this| {
                    let invalid = this.read_text(cx).to_lowercase() == "invalid";

                    this.invalid(invalid)
                }),
            )
            .child(Button::new("button").disabled(self.checkbox_checked || self.switch_checked))
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
                |_window, cx| {
                    cx.new(|cx| Root {
                        focus_handle: cx.focus_handle(),
                        checkbox_checked: false,
                        switch_checked: false,
                    })
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
