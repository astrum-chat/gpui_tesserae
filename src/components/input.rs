use gpui::{
    ElementId, Entity, Focusable, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, div, px,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_tesserae_theme::ThemeExt;
use gpui_transitions::{TransitionExt, TransitionGoal};

use crate::{
    primitives::{
        FocusRing,
        input::{Input as PrimitiveInput, InputState},
    },
    theme::ThemeLayerKind,
    utils::{
        ElementIdExt, PixelsExt, PositionalChildren, PositionalParentElement, RgbaExt,
        hover_and_focus_border_color_transition,
    },
};

#[derive(IntoElement)]
pub struct Input {
    id: ElementId,
    state: Entity<InputState>,
    layer: ThemeLayerKind,
    children: PositionalChildren,
}

impl Input {
    pub fn new(id: impl Into<ElementId>, state: Entity<InputState>) -> Self {
        Self {
            id: id.into(),
            state,
            layer: ThemeLayerKind::Tertiary,
            children: PositionalChildren::default(),
        }
    }
}

impl RenderOnce for Input {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let (primary_text_color, secondary_text_color) =
            cx.get_theme().variants.active().colors.text.all();
        let primary_accent_color = cx.get_theme().variants.active().colors.accent.primary;
        let background_color = *self.layer.resolve(cx.get_theme());
        let border_color = *self.layer.next().resolve(cx.get_theme());
        let border_hover_color = border_color.apply_delta(&primary_text_color, 0.07);
        let font_family = cx.get_theme().layout.text.default_font.family[0].clone();
        let line_height = cx.get_theme().layout.text.default_font.line_height;
        let text_size = cx.get_theme().layout.text.default_font.sizes.body.clone();
        let corner_radii = cx.get_theme().layout.corner_radii.md;
        let horizontal_padding = cx.get_theme().layout.padding.lg;
        let vertical_padding =
            cx.get_theme()
                .layout
                .size
                .lg
                .padding_needed_for_height(window, text_size, line_height);

        let is_focus = self.state.focus_handle(cx).is_focused(window);
        let is_hover_state = window.use_state(cx, |_cx, _window| false);
        let is_hover = *is_hover_state.read(cx);

        let border_color_transition_state = hover_and_focus_border_color_transition(
            self.id.with_suffix("state:border_color"),
            window,
            cx,
            is_hover,
            is_focus,
            border_color,
            border_hover_color,
            primary_accent_color,
        );

        div()
            .id(self.id.clone())
            .w_full()
            .h_auto()
            .pl(horizontal_padding)
            .pr(horizontal_padding)
            .pt(vertical_padding)
            .pb(vertical_padding)
            .gap(horizontal_padding)
            .flex()
            .flex_col()
            .on_hover(move |hover, _window, cx| {
                is_hover_state.update(cx, |this, _cx| *this = *hover);
                cx.notify(is_hover_state.entity_id());
            })
            .child(
                FocusRing::new(
                    self.id.with_suffix("focus_ring"),
                    self.state.focus_handle(cx).clone(),
                )
                .rounded(corner_radii),
            )
            .child(
                squircle()
                    .absolute_expand()
                    .rounded(corner_radii)
                    .bg(background_color)
                    .border(px(1.))
                    .border_inside()
                    .with_transitions(border_color_transition_state, move |_cx, this, color| {
                        this.border_color(color)
                    }),
            )
            .children(self.children.top)
            .child(
                div()
                    .w_full()
                    .flex()
                    .gap(horizontal_padding)
                    .items_center()
                    .children(self.children.left)
                    .child(
                        PrimitiveInput::new(self.state)
                            .w_full()
                            .text_size(text_size)
                            .font_family(font_family)
                            .text_color(primary_text_color)
                            .placeholder_text_color(secondary_text_color)
                            .selection_color(primary_accent_color.alpha(0.3))
                            .line_height(line_height),
                    )
                    .children(self.children.right),
            )
            .children(self.children.bottom)
    }
}

impl PositionalParentElement for Input {
    fn children_mut(&mut self) -> &mut crate::utils::PositionalChildren {
        &mut self.children
    }
}
