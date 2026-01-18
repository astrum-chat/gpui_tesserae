use std::time::Duration;

use gpui::{
    App, CursorStyle, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, ease_out_quint,
    prelude::FluentBuilder, px, relative, svg,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::Lerp;

use crate::{
    TesseraeIconKind, conitional_transition,
    primitives::FocusRing,
    theme::{ThemeExt, ThemeLayerKind},
    utils::{ElementIdExt, RgbaExt, SquircleExt, checked_transition, disabled_transition},
};

#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    icon: SharedString,
    layer: ThemeLayerKind,
    checked: bool,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
}

impl Checkbox {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            icon: TesseraeIconKind::Checkmark.into(),
            layer: ThemeLayerKind::Tertiary,
            checked: false,
            disabled: false,
            on_click: None,
        }
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = icon.into();
        self
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

    pub fn on_click(mut self, on_click: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(on_click));
        self
    }

    fn handle_on_click(
        window: &mut Window,
        cx: &mut App,
        checked: bool,
        on_click: Option<&Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    ) {
        if let Some(on_click) = on_click {
            (on_click)(&checked, window, cx)
        }
    }
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let size = cx.get_theme().layout.size.md;
        let corner_radius = cx.get_theme().layout.corner_radii.sm;
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
            Duration::from_millis(285),
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

        div()
            .id(self.id.clone())
            .cursor(if is_disabled {
                CursorStyle::OperationNotAllowed
            } else {
                CursorStyle::PointingHand
            })
            .size(size)
            .min_w(size)
            .min_h(size)
            .flex()
            .items_center()
            .justify_center()
            .opacity(*disabled_transition.evaluate(window, cx))
            .child(
                FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone())
                    .rounded(corner_radius),
            )
            .child(
                squircle()
                    .absolute_expand()
                    .rounded(corner_radius)
                    .bg(background_color)
                    .border(px(1.))
                    .border_inside()
                    .border_color(*border_color_transition.evaluate(window, cx)),
            )
            .map(|this| {
                let checked_delta = *checked_transition.evaluate(window, cx);

                this.child(
                    squircle()
                        .absolute_expand()
                        .rounded(corner_radius)
                        .border(px(1.))
                        .border_inside()
                        .bg(primary_accent_color.alpha(checked_delta))
                        .border_highlight(checked_delta * 0.15),
                )
                .child(
                    svg()
                        .map(|mut this| {
                            this.style().aspect_ratio = Some(1.);
                            this
                        })
                        .size(relative(0.48))
                        .text_color(primary_text_color.alpha(checked_delta))
                        .path(self.icon.clone()),
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
                .on_click({
                    move |_, window, cx| {
                        window.prevent_default();

                        if !is_focus {
                            // We only want to blur if something else may be focused.
                            window.blur();
                        }

                        is_click_down_state_on_click.update(cx, |this, _cx| *this = false);
                        cx.notify(is_click_down_state_on_click.entity_id());

                        Self::handle_on_click(window, cx, !self.checked, self.on_click.as_ref());
                    }
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

#[cfg(all(test, feature = "test-support"))]
mod tests {

    use super::*;
    use gpui::{AppContext, TestAppContext, VisualTestContext};

    #[gpui::test]
    fn test_checkbox_creation(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let checkbox = Checkbox::new("test-checkbox");
            assert!(!checkbox.checked, "Checkbox should start unchecked");
            assert!(!checkbox.disabled, "Checkbox should start enabled");
        });
    }

    #[gpui::test]
    fn test_checkbox_checked_state(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let checkbox = Checkbox::new("test-checkbox").checked(true);
            assert!(checkbox.checked, "Checkbox should be checked");

            let checkbox = Checkbox::new("test-checkbox").checked(false);
            assert!(!checkbox.checked, "Checkbox should be unchecked");
        });
    }

    #[gpui::test]
    fn test_checkbox_disabled_state(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let checkbox = Checkbox::new("test-checkbox").disabled(true);
            assert!(checkbox.disabled, "Checkbox should be disabled");

            let checkbox = Checkbox::new("test-checkbox").disabled(false);
            assert!(!checkbox.disabled, "Checkbox should be enabled");
        });
    }

    #[gpui::test]
    fn test_checkbox_layer(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let checkbox = Checkbox::new("test-checkbox").layer(ThemeLayerKind::Primary);
            assert!(
                matches!(checkbox.layer, ThemeLayerKind::Primary),
                "Checkbox should have primary layer"
            );

            let checkbox = Checkbox::new("test-checkbox").layer(ThemeLayerKind::Secondary);
            assert!(
                matches!(checkbox.layer, ThemeLayerKind::Secondary),
                "Checkbox should have secondary layer"
            );
        });
    }

    #[gpui::test]
    fn test_checkbox_custom_icon(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let custom_icon: SharedString = "custom/icon.svg".into();
            let checkbox = Checkbox::new("test-checkbox").icon(custom_icon.clone());
            assert_eq!(
                checkbox.icon, custom_icon,
                "Checkbox should have custom icon"
            );
        });
    }

    #[gpui::test]
    fn test_checkbox_builder_chain(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let checkbox = Checkbox::new("test-checkbox")
                .checked(true)
                .disabled(true)
                .layer(ThemeLayerKind::Secondary);

            assert!(checkbox.checked, "Checkbox should be checked");
            assert!(checkbox.disabled, "Checkbox should be disabled");
            assert!(
                matches!(checkbox.layer, ThemeLayerKind::Secondary),
                "Checkbox should have secondary layer"
            );
        });
    }

    #[gpui::test]
    fn test_checkbox_on_click_callback(cx: &mut TestAppContext) {
        use std::cell::Cell;
        use std::rc::Rc;

        let clicked = Rc::new(Cell::new(false));
        let clicked_value = Rc::new(Cell::new(false));

        cx.update(|_cx| {
            let clicked_clone = clicked.clone();
            let clicked_value_clone = clicked_value.clone();

            let checkbox = Checkbox::new("test-checkbox").on_click(move |value, _window, _cx| {
                clicked_clone.set(true);
                clicked_value_clone.set(*value);
            });

            assert!(
                checkbox.on_click.is_some(),
                "Checkbox should have on_click callback"
            );
        });
    }

    #[gpui::test]
    fn test_checkbox_renders_in_window(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};

        let window = cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            cx.open_window(Default::default(), |_window, cx| {
                cx.new(|_cx| CheckboxTestView { checked: false })
            })
            .unwrap()
        });

        let _cx = VisualTestContext::from_window(window.into(), cx);

        // The window creation itself validates rendering works
    }

    /// Test view that contains a Checkbox
    struct CheckboxTestView {
        checked: bool,
    }

    impl gpui::Render for CheckboxTestView {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            div().size_full().child(
                Checkbox::new("test-checkbox")
                    .checked(self.checked)
                    .on_click(cx.listener(|view, checked, _window, cx| {
                        view.checked = *checked;
                        cx.notify();
                    })),
            )
        }
    }
}
