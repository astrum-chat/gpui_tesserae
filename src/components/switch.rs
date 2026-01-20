use std::time::Duration;

use gpui::{
    CursorStyle, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, div, ease_out_quint, prelude::FluentBuilder, px,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::Lerp;

use crate::{
    ElementIdExt, conitional_transition,
    primitives::{ClickHandlers, Clickable, FocusRing},
    theme::{ThemeExt, ThemeLayerKind},
    utils::{RgbaExt, SquircleExt, checked_transition, disabled_transition},
};

#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    layer: ThemeLayerKind,
    checked: bool,
    disabled: bool,
    click_handlers: ClickHandlers,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            layer: ThemeLayerKind::Tertiary,
            checked: false,
            disabled: false,
            click_handlers: ClickHandlers::new(),
        }
    }

    pub fn layer(mut self, layer: ThemeLayerKind) -> Self {
        self.layer = layer;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Clickable for Switch {
    fn click_handlers_mut(&mut self) -> &mut ClickHandlers {
        &mut self.click_handlers
    }
}

impl RenderOnce for Switch {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        const INNER_SIZE_FOCUS_MULT: f32 = 1.25;

        let inner_size = cx.get_theme().layout.size.md;
        let padding = cx.get_theme().layout.padding.md;
        let width = (inner_size * 2) + (padding * 2);
        let height = inner_size + (padding * 2);
        let (start_offset, end_offset) = (
            padding.to_f64() as f32,
            (width - inner_size - padding).to_f64() as f32,
        );
        let primary_accent_color = cx.get_theme().variants.active(cx).colors.accent.primary;
        let primary_text_color = cx.get_theme().variants.active(cx).colors.text.primary;
        let background_color = self.layer.resolve(cx);
        let border_color = self.layer.next().resolve(cx);
        let border_hover_color = border_color.lerp(&primary_text_color, 0.07);
        let border_click_down_color = border_color.lerp(&primary_text_color, 0.16);

        let checked_transition = checked_transition(
            self.id.clone(),
            window,
            cx,
            Duration::from_millis(200),
            self.checked,
        );

        let is_disabled = self.disabled;

        let is_hover_state =
            window.use_keyed_state(self.id.with_suffix("state:hover"), cx, |_cx, _window| false);
        let is_hover = *is_hover_state.read(cx);

        let is_click_down_state = window.use_keyed_state(
            self.id.with_suffix("state:click_down"),
            cx,
            |_cx, _window| false,
        );
        let is_click_down = *is_click_down_state.read(cx);

        let focus_handle = window
            .use_keyed_state(
                self.id.with_suffix("state:focus_handle"),
                cx,
                |_window, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();
        let is_focus = focus_handle.is_focused(window);

        let disabled_transition = disabled_transition(self.id.clone(), window, cx, is_disabled);

        if is_focus && is_disabled {
            window.blur();
        }

        let border_color_transition = conitional_transition!(
            self.id.with_suffix("state:transition:border_color"),
            window,
            cx,
            Duration::from_millis(365),
            {
                is_focus => primary_accent_color,
                is_click_down => border_click_down_color,
                is_hover => border_hover_color,
                _ => border_color
            }
        )
        .with_easing(ease_out_quint());

        // We want the width of the inner circle to expand slightly when focused.
        let inner_width_transition = conitional_transition!(
            self.id.with_suffix("state:transition:inner_width"),
            window,
            cx,
            Duration::from_millis(185),
            {
                is_focus | is_click_down => px((inner_size.to_f64() as f32 * INNER_SIZE_FOCUS_MULT).floor()),
                _ => inner_size
            }
        )
        .with_easing(ease_out_quint());

        div()
            .id(self.id.clone())
            .cursor(if is_disabled {
                CursorStyle::OperationNotAllowed
            } else {
                CursorStyle::PointingHand
            })
            .w(width)
            .min_w(width)
            .h(height)
            .min_h(height)
            .opacity(*disabled_transition.evaluate(window, cx))
            .child(
                FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone())
                    .rounded(px(100.)),
            )
            .child(
                squircle()
                    .absolute_expand()
                    .rounded(px(100.))
                    .bg(background_color)
                    .border(px(1.))
                    .border_inside()
                    .border_color(*border_color_transition.evaluate(window, cx)),
            )
            .map(|this| {
                let checked_delta = *checked_transition.evaluate(window, cx);
                let inner_width = *inner_width_transition.evaluate(window, cx);

                let offset = remap(checked_delta, 0., 1., start_offset, end_offset);

                let width_diff = (inner_width - inner_size) * checked_delta;

                this.child(
                    squircle()
                        .absolute_expand()
                        .bg(primary_accent_color.alpha(checked_delta))
                        .rounded(px(100.))
                        .border_inside()
                        .border(px(1.))
                        .border_highlight(0.15 * checked_delta),
                )
                .child(
                    div()
                        .w(inner_width)
                        .h(inner_size)
                        .top(padding)
                        .bg(primary_text_color)
                        .rounded(px(100.))
                        .left(px(offset) - width_diff),
                )
            })
            .when(!is_disabled, |this| {
                let is_hover_state_on_hover = is_hover_state.clone();
                let is_click_down_state_on_mouse_down = is_click_down_state.clone();
                let is_click_down_state_on_click = is_click_down_state.clone();

                this.on_hover(move |hover, _window, cx| {
                    is_hover_state_on_hover.update(cx, |this, _cx| *this = *hover);
                    cx.notify(is_hover_state_on_hover.entity_id());
                })
                .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                    // Prevents focus ring from appearing when clicked.
                    window.prevent_default();

                    is_click_down_state_on_mouse_down.update(cx, |this, _cx| *this = true);
                    cx.notify(is_click_down_state_on_mouse_down.entity_id());
                })
                .map(|mut this| {
                    if let Some((button, handler)) = self.click_handlers.on_mouse_down {
                        if button != gpui::MouseButton::Left {
                            this = this.on_mouse_down(button, move |event, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                                (handler)(event, window, cx);
                            });
                        }
                    }

                    if let Some((button, handler)) = self.click_handlers.on_mouse_up {
                        this = this.on_mouse_up(button, move |event, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            (handler)(event, window, cx);
                        });
                    }

                    if let Some(handler) = self.click_handlers.on_any_mouse_down {
                        this = this.on_any_mouse_down(move |event, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            (handler)(event, window, cx);
                        });
                    }

                    if let Some(handler) = self.click_handlers.on_any_mouse_up {
                        this.interactivity()
                            .on_any_mouse_up(move |event, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                                (handler)(event, window, cx);
                            });
                    }

                    let on_click = self.click_handlers.on_click;
                    this.on_click(move |event, window, cx| {
                        window.prevent_default();
                        cx.stop_propagation();

                        if !is_focus {
                            // We only want to blur if something else may be focused.
                            window.blur();
                        }

                        is_click_down_state_on_click.update(cx, |this, _cx| *this = false);
                        cx.notify(is_click_down_state_on_click.entity_id());

                        if let Some(on_click) = &on_click {
                            (on_click)(event, window, cx);
                        }
                    })
                })
                .on_mouse_up_out(gpui::MouseButton::Left, move |_event, _window, cx| {
                    // We need to clean up states when the mouse clicks down on the component, leaves its bounds, then unclicks.

                    is_hover_state.update(cx, |this, _cx| *this = false);
                    cx.notify(is_hover_state.entity_id());

                    is_click_down_state.update(cx, |this, _cx| *this = false);
                    cx.notify(is_click_down_state.entity_id());
                })
                .track_focus(&focus_handle)
            })
    }
}

pub fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    (value - from_min) / (from_max - from_min) * (to_max - to_min) + to_min
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, VisualTestContext};

    #[gpui::test]
    fn test_switch_creation(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let switch = Switch::new("test-switch");
            assert!(!switch.checked, "Switch should start unchecked");
            assert!(!switch.disabled, "Switch should start enabled");
        });
    }

    #[gpui::test]
    fn test_switch_checked_state(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let switch = Switch::new("test-switch").checked(true);
            assert!(switch.checked, "Switch should be checked");

            let switch = Switch::new("test-switch").checked(false);
            assert!(!switch.checked, "Switch should be unchecked");
        });
    }

    #[gpui::test]
    fn test_switch_disabled_state(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let switch = Switch::new("test-switch").disabled(true);
            assert!(switch.disabled, "Switch should be disabled");

            let switch = Switch::new("test-switch").disabled(false);
            assert!(!switch.disabled, "Switch should be enabled");
        });
    }

    #[gpui::test]
    fn test_switch_layer(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let switch = Switch::new("test-switch").layer(ThemeLayerKind::Primary);
            assert!(
                matches!(switch.layer, ThemeLayerKind::Primary),
                "Switch should have primary layer"
            );

            let switch = Switch::new("test-switch").layer(ThemeLayerKind::Secondary);
            assert!(
                matches!(switch.layer, ThemeLayerKind::Secondary),
                "Switch should have secondary layer"
            );
        });
    }

    #[gpui::test]
    fn test_switch_builder_chain(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let switch = Switch::new("test-switch")
                .checked(true)
                .disabled(true)
                .layer(ThemeLayerKind::Secondary);

            assert!(switch.checked, "Switch should be checked");
            assert!(switch.disabled, "Switch should be disabled");
            assert!(
                matches!(switch.layer, ThemeLayerKind::Secondary),
                "Switch should have secondary layer"
            );
        });
    }

    #[gpui::test]
    fn test_switch_on_click_callback(cx: &mut TestAppContext) {
        use crate::primitives::Clickable;

        cx.update(|_cx| {
            let switch = Switch::new("test-switch").on_click(move |_event, _window, _cx| {});

            assert!(
                switch.click_handlers.on_click.is_some(),
                "Switch should have on_click callback"
            );
        });
    }

    #[gpui::test]
    fn test_switch_on_any_mouse_down_callback(cx: &mut TestAppContext) {
        use crate::primitives::Clickable;

        cx.update(|_cx| {
            let switch =
                Switch::new("test-switch").on_any_mouse_down(move |_event, _window, _cx| {});

            assert!(
                switch.click_handlers.on_any_mouse_down.is_some(),
                "Switch should have on_any_mouse_down callback"
            );
        });
    }

    #[gpui::test]
    fn test_switch_on_any_mouse_up_callback(cx: &mut TestAppContext) {
        use crate::primitives::Clickable;

        cx.update(|_cx| {
            let switch = Switch::new("test-switch").on_any_mouse_up(move |_event, _window, _cx| {});

            assert!(
                switch.click_handlers.on_any_mouse_up.is_some(),
                "Switch should have on_any_mouse_up callback"
            );
        });
    }

    #[gpui::test]
    fn test_switch_renders_in_window(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};

        let window = cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            cx.open_window(Default::default(), |_window, cx| {
                cx.new(|_cx| SwitchTestView { checked: false })
            })
            .unwrap()
        });

        let _cx = VisualTestContext::from_window(window.into(), cx);

        // The window creation itself validates rendering works
    }

    #[gpui::test]
    fn test_remap_function(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            // Test remap from 0-1 to 0-100
            assert_eq!(remap(0.0, 0.0, 1.0, 0.0, 100.0), 0.0);
            assert_eq!(remap(0.5, 0.0, 1.0, 0.0, 100.0), 50.0);
            assert_eq!(remap(1.0, 0.0, 1.0, 0.0, 100.0), 100.0);

            // Test remap from 0-10 to 0-1
            assert_eq!(remap(0.0, 0.0, 10.0, 0.0, 1.0), 0.0);
            assert_eq!(remap(5.0, 0.0, 10.0, 0.0, 1.0), 0.5);
            assert_eq!(remap(10.0, 0.0, 10.0, 0.0, 1.0), 1.0);

            // Test remap with negative ranges
            assert_eq!(remap(0.0, -1.0, 1.0, 0.0, 100.0), 50.0);
        });
    }

    /// Test view that contains a Switch
    struct SwitchTestView {
        checked: bool,
    }

    impl gpui::Render for SwitchTestView {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            div()
                .size_full()
                .child(Switch::new("test-switch").checked(self.checked))
        }
    }
}
