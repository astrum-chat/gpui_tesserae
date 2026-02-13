use std::time::Duration;

use gpui::{
    CursorStyle, Edges, ElementId, FocusHandle, InteractiveElement, IntoElement, Length,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, div,
    ease_out_quint, prelude::FluentBuilder, px, relative, svg,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::Lerp;

use crate::{
    TesseraeIconKind, conitional_transition,
    extensions::{
        mouse_behavior::{MouseBehavior, MouseBehaviorExt},
        mouse_handleable::{MouseHandleable, MouseHandlers},
    },
    primitives::FocusRing,
    theme::{ThemeExt, ThemeLayerKind},
    utils::{ElementIdExt, RgbaExt, SquircleExt, checked_transition, disabled_transition},
};

/// A checkbox component with animated check state transitions.
#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    icon: SharedString,
    layer: ThemeLayerKind,
    checked: bool,
    disabled: bool,
    force_hover: bool,
    focus_handle: Option<FocusHandle>,
    on_hover: Option<Box<dyn Fn(&bool, &mut gpui::Window, &mut gpui::App) + 'static>>,
    mouse_handlers: MouseHandlers<bool>,
    mouse_behavior: MouseBehavior,
    margin: Edges<Option<Length>>,
}

impl Checkbox {
    /// Creates a new checkbox with the given element ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            icon: TesseraeIconKind::Checkmark.into(),
            layer: ThemeLayerKind::Tertiary,
            checked: false,
            disabled: false,
            force_hover: false,
            focus_handle: None,
            on_hover: None,
            mouse_handlers: MouseHandlers::new(),
            mouse_behavior: MouseBehavior::default(),
            margin: Edges::default(),
        }
    }

    /// Sets uniform margin for all sides.
    pub fn m(mut self, margin: impl Into<Length>) -> Self {
        let margin = margin.into();
        self.margin = Edges::all(Some(margin));
        self
    }

    /// Sets top margin.
    pub fn mt(mut self, margin: impl Into<Length>) -> Self {
        self.margin.top = Some(margin.into());
        self
    }

    /// Sets bottom margin.
    pub fn mb(mut self, margin: impl Into<Length>) -> Self {
        self.margin.bottom = Some(margin.into());
        self
    }

    /// Sets left margin.
    pub fn ml(mut self, margin: impl Into<Length>) -> Self {
        self.margin.left = Some(margin.into());
        self
    }

    /// Sets right margin.
    pub fn mr(mut self, margin: impl Into<Length>) -> Self {
        self.margin.right = Some(margin.into());
        self
    }

    /// Sets the focus handle for keyboard navigation.
    pub fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle);
        self
    }

    /// Sets a custom icon to display when checked.
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Sets the background layer for theming depth.
    pub fn layer(mut self, layer: ThemeLayerKind) -> Self {
        self.layer = layer;
        self
    }

    /// Sets the checked state.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Sets the disabled state, preventing interaction.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Forces the hover visual state regardless of actual hover.
    pub fn force_hover(mut self, force_hover: bool) -> Self {
        self.force_hover = force_hover;
        self
    }

    /// Sets a callback invoked when hover state changes.
    pub fn on_hover(
        mut self,
        on_hover: impl Fn(&bool, &mut gpui::Window, &mut gpui::App) + 'static,
    ) -> Self {
        self.on_hover = Some(Box::new(on_hover));
        self
    }
}

impl MouseHandleable<bool> for Checkbox {
    fn mouse_handlers_mut(&mut self) -> &mut MouseHandlers<bool> {
        &mut self.mouse_handlers
    }
}

impl MouseBehaviorExt for Checkbox {
    fn mouse_behavior_mut(&mut self) -> &mut MouseBehavior {
        &mut self.mouse_behavior
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
        let is_hover = self.force_hover || *is_hover_state.read(cx);

        let is_click_down_state = window.use_keyed_state(
            self.id.with_suffix("state:click_down"),
            cx,
            |_cx, _window| false,
        );
        let is_click_down = *is_click_down_state.read(cx);

        let focus_handle = self
            .focus_handle
            .as_ref()
            .unwrap_or_else(|| {
                window
                    .use_keyed_state(
                        self.id.with_suffix("state:focus_handle"),
                        cx,
                        |_window, cx| cx.focus_handle().tab_stop(true),
                    )
                    .read(cx)
            })
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
            .when_some(self.margin.top, |this, v| this.mt(v))
            .when_some(self.margin.bottom, |this, v| this.mb(v))
            .when_some(self.margin.left, |this, v| this.ml(v))
            .when_some(self.margin.right, |this, v| this.mr(v))
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
                let behavior = self.mouse_behavior;
                let checked = self.checked;

                this.on_hover(move |hover, window, cx| {
                    is_hover_state_on_hover.update(cx, |this, cx| {
                        *this = *hover;
                        cx.notify();
                    });

                    if let Some(callback) = self.on_hover.as_ref() {
                        (callback)(hover, window, cx);
                    }
                })
                .on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                    behavior.apply(window, cx);

                    is_click_down_state_on_mouse_down.update(cx, |this, cx| {
                        *this = true;
                        cx.notify();
                    });
                })
                .map(|mut this| {
                    let behavior = self.mouse_behavior;

                    if let Some((button, handler)) = self.mouse_handlers.on_mouse_down {
                        if button != gpui::MouseButton::Left {
                            this = this.on_mouse_down(button, move |event, window, cx| {
                                behavior.apply(window, cx);
                                (handler)(event, window, cx);
                            });
                        }
                    }

                    if let Some((button, handler)) = self.mouse_handlers.on_mouse_up {
                        this = this.on_mouse_up(button, move |event, window, cx| {
                            behavior.apply(window, cx);
                            (handler)(event, window, cx);
                        });
                    }

                    if let Some(handler) = self.mouse_handlers.on_any_mouse_down {
                        this = this.on_any_mouse_down(move |event, window, cx| {
                            behavior.apply(window, cx);
                            (handler)(event, window, cx);
                        });
                    }

                    if let Some(handler) = self.mouse_handlers.on_any_mouse_up {
                        this.interactivity()
                            .on_any_mouse_up(move |event, window, cx| {
                                behavior.apply(window, cx);
                                (handler)(event, window, cx);
                            });
                    }

                    let on_click = self.mouse_handlers.on_click;
                    this.on_click(move |_event, window, cx| {
                        behavior.apply(window, cx);

                        if !is_focus {
                            window.blur();
                        }

                        is_click_down_state_on_click.update(cx, |this, cx| {
                            *this = false;
                            cx.notify();
                        });

                        if let Some(on_click) = &on_click {
                            let new_checked = !checked;
                            (on_click)(&new_checked, window, cx);
                        }
                    })
                })
                .on_mouse_up_out(gpui::MouseButton::Left, move |_event, _window, cx| {
                    // We need to clean up states when the mouse clicks down on the component, leaves its bounds, then unclicks.

                    is_hover_state.update(cx, |this, cx| {
                        *this = false;
                        cx.notify();
                    });

                    is_click_down_state.update(cx, |this, cx| {
                        *this = false;
                        cx.notify();
                    });
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
        use crate::extensions::mouse_handleable::MouseHandleable;

        cx.update(|_cx| {
            let checkbox = Checkbox::new("test-checkbox").on_click(move |_event, _window, _cx| {});

            assert!(
                checkbox.mouse_handlers.on_click.is_some(),
                "Checkbox should have on_click callback"
            );
        });
    }

    #[gpui::test]
    fn test_checkbox_on_any_mouse_down_callback(cx: &mut TestAppContext) {
        use crate::extensions::mouse_handleable::MouseHandleable;

        cx.update(|_cx| {
            let checkbox =
                Checkbox::new("test-checkbox").on_any_mouse_down(move |_event, _window, _cx| {});

            assert!(
                checkbox.mouse_handlers.on_any_mouse_down.is_some(),
                "Checkbox should have on_any_mouse_down callback"
            );
        });
    }

    #[gpui::test]
    fn test_checkbox_on_any_mouse_up_callback(cx: &mut TestAppContext) {
        use crate::extensions::mouse_handleable::MouseHandleable;

        cx.update(|_cx| {
            let checkbox =
                Checkbox::new("test-checkbox").on_any_mouse_up(move |_event, _window, _cx| {});

            assert!(
                checkbox.mouse_handlers.on_any_mouse_up.is_some(),
                "Checkbox should have on_any_mouse_up callback"
            );
        });
    }

    #[gpui::test]
    fn test_checkbox_on_hover_callback(cx: &mut TestAppContext) {
        use std::cell::Cell;
        use std::rc::Rc;

        let hovered = Rc::new(Cell::new(false));

        cx.update(|_cx| {
            let hovered_clone = hovered.clone();

            let checkbox =
                Checkbox::new("test-checkbox").on_hover(move |is_hover, _window, _cx| {
                    hovered_clone.set(*is_hover);
                });

            assert!(
                checkbox.on_hover.is_some(),
                "Checkbox should have on_hover callback"
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

    #[gpui::test]
    fn test_checkbox_mouse_behavior_default(cx: &mut TestAppContext) {
        use crate::extensions::mouse_behavior::MouseBehaviorExt;

        cx.update(|_cx| {
            let mut checkbox = Checkbox::new("test-checkbox");
            let behavior = checkbox.mouse_behavior_mut();

            assert!(
                !behavior.allow_propagation,
                "Checkbox should not allow propagation by default"
            );
            assert!(
                !behavior.allow_default,
                "Checkbox should not allow default by default"
            );
        });
    }

    #[gpui::test]
    fn test_checkbox_allow_mouse_propagation(cx: &mut TestAppContext) {
        use crate::extensions::mouse_behavior::MouseBehaviorExt;

        cx.update(|_cx| {
            let mut checkbox = Checkbox::new("test-checkbox").allow_mouse_propagation();
            let behavior = checkbox.mouse_behavior_mut();

            assert!(
                behavior.allow_propagation,
                "Checkbox should allow propagation after calling allow_mouse_propagation"
            );
            assert!(
                !behavior.allow_default,
                "Checkbox should still not allow default"
            );
        });
    }

    #[gpui::test]
    fn test_checkbox_allow_default_mouse_behaviour(cx: &mut TestAppContext) {
        use crate::extensions::mouse_behavior::MouseBehaviorExt;

        cx.update(|_cx| {
            let mut checkbox = Checkbox::new("test-checkbox").allow_default_mouse_behaviour();
            let behavior = checkbox.mouse_behavior_mut();

            assert!(
                !behavior.allow_propagation,
                "Checkbox should still not allow propagation"
            );
            assert!(
                behavior.allow_default,
                "Checkbox should allow default after calling allow_default_mouse_behaviour"
            );
        });
    }

    #[gpui::test]
    fn test_checkbox_mouse_behavior_chain(cx: &mut TestAppContext) {
        use crate::extensions::mouse_behavior::MouseBehaviorExt;

        cx.update(|_cx| {
            let mut checkbox = Checkbox::new("test-checkbox")
                .allow_mouse_propagation()
                .allow_default_mouse_behaviour();
            let behavior = checkbox.mouse_behavior_mut();

            assert!(
                behavior.allow_propagation,
                "Checkbox should allow propagation"
            );
            assert!(behavior.allow_default, "Checkbox should allow default");
        });
    }

    /// Test view that contains a Checkbox
    struct CheckboxTestView {
        checked: bool,
    }

    impl gpui::Render for CheckboxTestView {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            div()
                .size_full()
                .child(Checkbox::new("test-checkbox").checked(self.checked))
        }
    }
}
