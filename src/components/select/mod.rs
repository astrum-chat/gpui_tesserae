use std::{sync::Arc, time::Duration};

use gpui::{
    ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, div, ease_out_quint, prelude::FluentBuilder, px, radians,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::Lerp;

use crate::{
    ElementIdExt, TesseraeIconKind,
    components::Icon,
    conitional_transition,
    primitives::FocusRing,
    theme::{ThemeExt, ThemeLayerKind},
    utils::{PixelsExt, disabled_transition},
};

mod menu;
pub use menu::*;

mod item;
pub use item::*;

mod state;
pub use state::*;

#[derive(IntoElement)]
pub struct Select<V: 'static, I: SelectItem<Value = V> + 'static> {
    id: ElementId,
    disabled: bool,
    layer: ThemeLayerKind,
    state: Arc<SelectState<V, I>>,
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> Select<V, I> {
    pub fn new(id: impl Into<ElementId>, state: impl Into<Arc<SelectState<V, I>>>) -> Self {
        Self {
            id: id.into(),
            disabled: false,
            layer: ThemeLayerKind::Tertiary,
            state: state.into(),
        }
    }
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> RenderOnce for Select<V, I> {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let (primary_text_color, secondary_text_color) =
            cx.get_theme().variants.active(cx).colors.text.all();
        let primary_accent_color = cx.get_theme().variants.active(cx).colors.accent.primary;
        let background_color = self.layer.resolve(cx);
        let border_color = self.layer.next().resolve(cx);
        let border_hover_color = border_color.lerp(&primary_text_color, 0.07);
        let font_family = cx.get_theme().layout.text.default_font.family[0].clone();
        let line_height = cx.get_theme().layout.text.default_font.line_height;
        let text_size = /*self
            .style
            .text_size
            .unwrap_or_else(|| */cx.get_theme().layout.text.default_font.sizes.body.clone()/*)*/;
        let corner_radius = cx.get_theme().layout.corner_radii.md;
        //let corner_radii_override = self.style.corner_radii;
        //let padding_override = self.style.padding;
        // let inner_padding_override = self.style.inner_padding;
        let horizontal_padding = cx.get_theme().layout.padding.lg;
        let vertical_padding =
            cx.get_theme()
                .layout
                .size
                .lg
                .padding_needed_for_height(window, text_size, line_height);

        let is_hover_state =
            window.use_keyed_state(self.id.with_suffix("state:hover"), cx, |_cx, _window| false);
        let is_hover = *is_hover_state.read(cx);

        let focus_handle = window
            .use_keyed_state(
                self.id.with_suffix("state:focus_handle"),
                cx,
                |_window, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();
        let is_focus = focus_handle.is_focused(window);

        let is_disabled = self.disabled;
        let disabled_transition = disabled_transition(self.id.clone(), window, cx, is_disabled);

        if is_focus && is_disabled {
            window.blur();
        }

        let border_color_transition = conitional_transition!(
            self.id.with_suffix("state:transition:border_color"),
            window,
            cx,
            Duration::from_millis(400),
            {
                is_focus => primary_accent_color,
                is_hover => border_hover_color,
                _ => border_color
            }
        )
        .with_easing(ease_out_quint());

        let menu_visible_transition = conitional_transition!(
            self.id.with_suffix("state:transition:menu_visible"),
            window,
            cx,
            Duration::from_millis(350),
            {
                is_focus => 1.,
                _ => 0.
            }
        )
        .with_easing(ease_out_quint());

        let menu_visible_delta = *menu_visible_transition.evaluate(window, cx);

        div()
            .id(self.id.clone())
            .track_focus(&focus_handle)
            .cursor_pointer()
            .w_full()
            .h_auto()
            .pl(horizontal_padding)
            .pr(horizontal_padding)
            .pt(vertical_padding)
            .pb(vertical_padding)
            .gap(horizontal_padding)
            .flex()
            .flex_col()
            .map(|this| {
                let focus_handle = focus_handle.clone();
                let disabled_delta = *disabled_transition.evaluate(window, cx);

                this.opacity(disabled_delta).child(
                    FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone())
                        .rounded(corner_radius),
                )
            })
            .child(
                squircle()
                    .absolute_expand()
                    .rounded(corner_radius)
                    .bg(background_color)
                    .border(px(1.))
                    .border_inside()
                    .border_color(*border_color_transition.evaluate(window, cx)),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(text_size)
                    .text_color(secondary_text_color)
                    .font_family(font_family.clone())
                    .map(|this| {
                        let Some(item_name) = self.state.selected_item.read(cx) else {
                            return this.child("No item selected");
                        };

                        let Some(item) = self.state.items.read(cx).get(item_name) else {
                            return this.child("No item selected");
                        };

                        this.child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .text_size(text_size)
                                .text_color(primary_text_color)
                                .font_family(font_family)
                                .child(item.display(window, cx)),
                        )
                    })
                    .child(
                        Icon::new(TesseraeIconKind::ArrowDown)
                            .size(px(11.))
                            .color(secondary_text_color)
                            .map(|this| {
                                let rotation = radians(
                                    ((1. - menu_visible_delta) * 180.) * std::f32::consts::PI
                                        / 180.0,
                                );

                                this.rotate(rotation)
                            }),
                    ),
            )
            .when(menu_visible_delta != 0., |this| {
                this.child(
                    div()
                        .w_full()
                        .absolute()
                        .top_full()
                        .left_0()
                        .pt(cx.get_theme().layout.padding.md)
                        .child(
                            SelectMenu::new(self.id.with_suffix("menu"), self.state.clone())
                                .opacity(menu_visible_delta),
                        ),
                )
            })
            .when(!is_disabled, |this| {
                this.on_hover(move |hover, _window, cx| {
                    is_hover_state.update(cx, |this, _cx| *this = *hover);
                    cx.notify(is_hover_state.entity_id());
                })
                .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    // We want to disable the default focus / blur behaviour.
                    window.prevent_default();
                    focus_handle.focus(window, cx);
                })
            })
    }
}
