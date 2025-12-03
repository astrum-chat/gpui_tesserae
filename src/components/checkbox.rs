use std::time::Duration;

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div, ease_out_quint, prelude::FluentBuilder, px,
    relative, svg,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::{Transition, TransitionExt, TransitionGoal};

use crate::{
    TesseraeIconKind,
    primitives::FocusRing,
    theme::{ThemeExt, ThemeLayerKind},
    utils::{ElementIdExt, RgbaExt, SquircleExt, hover_border_color_transition},
};

#[derive(IntoElement)]
pub struct Checkbox {
    id: ElementId,
    layer: ThemeLayerKind,
    checked: bool,
    disabled: bool,
    on_click: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
}

impl Checkbox {
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

    fn checked_transition(&self, window: &mut Window, cx: &mut App) -> Transition<f32> {
        let checked_float = self.checked as u8 as f32;

        let checked = Transition::new(
            self.id.with_suffix("state:checked"),
            window,
            cx,
            Duration::from_millis(300),
            |_cx, _window| checked_float,
        )
        .with_easing(ease_out_quint());

        let changed = checked.set(cx, checked_float);
        if changed {
            cx.notify(checked.entity_id());
        }

        checked
    }

    fn handle_on_click(&self, window: &mut Window, cx: &mut App) {
        if let Some(on_click) = self.on_click.as_ref() {
            (on_click)(&!self.checked, window, cx)
        }
    }
}

impl RenderOnce for Checkbox {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl gpui::IntoElement {
        let size = cx.get_theme().layout.size.md;
        let corner_radii = cx.get_theme().layout.corner_radii.sm;
        let primary_accent_color = cx.get_theme().variants.active().colors.accent.primary;
        let primary_text_color = cx.get_theme().variants.active().colors.text.primary;
        let background_color = *self.layer.resolve(cx.get_theme());
        let border_color = *self.layer.next().resolve(cx.get_theme());
        let border_hover_color = border_color.apply_delta(&primary_text_color, 0.07);

        let checked_state = self.checked_transition(window, cx);

        let is_hover_state = window.use_state(cx, |_cx, _window| false);
        let is_hover = *is_hover_state.read(cx);

        let border_color_transition_state = hover_border_color_transition(
            self.id.with_suffix("state:border_color"),
            window,
            cx,
            is_hover,
            border_color,
            border_hover_color,
        );

        let focus_handle_state = window
            .use_keyed_state(
                self.id.with_suffix("state:focus_handle"),
                cx,
                |_window, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx);

        div()
            .id(self.id.clone())
            .cursor_pointer()
            .size(size)
            .min_w(size)
            .min_h(size)
            .flex()
            .items_center()
            .justify_center()
            .on_hover(move |hover, _window, cx| {
                is_hover_state.update(cx, |this, _cx| *this = *hover);
                cx.notify(is_hover_state.entity_id());
            })
            .child(
                FocusRing::new(
                    self.id.with_suffix("focus_ring"),
                    focus_handle_state.clone(),
                )
                .rounded(corner_radii),
            )
            .child(
                squircle()
                    .absolute_expand()
                    .rounded(corner_radii.clone())
                    .bg(background_color)
                    .border(px(1.))
                    .border_inside()
                    .with_transitions(border_color_transition_state, move |_cx, this, color| {
                        this.border_color(color)
                    }),
            )
            .with_transitions(checked_state, move |_cx, this, delta| {
                this.child(
                    squircle()
                        .absolute_expand()
                        .rounded(corner_radii.clone())
                        .border(px(1.))
                        .border_inside()
                        .bg(primary_accent_color.alpha(delta))
                        .border_highlight_color(delta * 0.15),
                )
                .child(
                    svg()
                        .map(|mut this| {
                            this.style().aspect_ratio = Some(1.);
                            this
                        })
                        .size(relative(0.48))
                        .text_color(primary_text_color.alpha(delta))
                        .path(TesseraeIconKind::Checkmark),
                )
            })
            .on_mouse_down(gpui::MouseButton::Left, |_, window, _| {
                // Prevents focusing.
                window.prevent_default();
            })
            .when(!self.disabled, |this| {
                this.on_click({
                    move |_, window, cx| {
                        window.prevent_default();
                        window.blur();
                        self.handle_on_click(window, cx);
                    }
                })
                .track_focus(focus_handle_state)
            })
    }
}
