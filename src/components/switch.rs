use std::time::Duration;

use gpui::{
    App, CursorStyle, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, ease_out_quint, prelude::FluentBuilder, px,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::{TransitionExt, TransitionGoal};

use crate::{
    ElementIdExt, conitional_transition,
    primitives::FocusRing,
    theme::{ThemeExt, ThemeLayerKind},
    utils::{RgbaExt, SquircleExt, checked_transition, disabled_transition},
};

#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    layer: ThemeLayerKind,
    checked: bool,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            layer: ThemeLayerKind::Tertiary,
            checked: false,
            disabled: false,
            on_click: None,
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

impl RenderOnce for Switch {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        const INNER_SIZE_FOCUS_MULT: f32 = 1.25;

        let inner_size = cx.get_theme().layout.size.md;
        let padding = px(4.);
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
        let border_hover_color = border_color.apply_delta(&primary_text_color, 0.07);
        let border_click_down_color = border_color.apply_delta(&primary_text_color, 0.16);

        let checked_state = checked_transition(
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

        let disabled_transition_state =
            disabled_transition(self.id.clone(), window, cx, is_disabled);

        if is_focus && is_disabled {
            window.blur();
        }

        let border_color_transition_state = conitional_transition!(
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
        let inner_width_state = conitional_transition!(
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
            .with_transitions(disabled_transition_state, move |_cx, this, opacity| {
                this.opacity(opacity)
            })
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
                    .with_transitions(border_color_transition_state, move |_cx, this, color| {
                        this.border_color(color)
                    }),
            )
            .with_transitions(
                (checked_state, inner_width_state),
                move |_cx, this, (delta, inner_width)| {
                    let offset = remap(delta, 0., 1., start_offset, end_offset);

                    let width_diff = (inner_width - inner_size) * delta;

                    this.child(
                        squircle()
                            .absolute_expand()
                            .bg(primary_accent_color.alpha(delta))
                            .rounded(px(100.))
                            .border_inside()
                            .border(px(1.))
                            .border_highlight_color(0.15 * delta),
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
                },
            )
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

pub fn remap(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    (value - from_min) / (from_max - from_min) * (to_max - to_min) + to_min
}
