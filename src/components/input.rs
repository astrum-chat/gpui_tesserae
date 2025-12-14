use std::time::Duration;

use gpui::{
    AbsoluteLength, App, Corners, DefiniteLength, Edges, ElementId, Entity, FocusHandle, Focusable,
    Hsla, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, div, ease_out_quint, prelude::FluentBuilder, px,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::{TransitionExt, TransitionGoal};

use crate::{
    conitional_transition,
    primitives::{
        FocusRing,
        input::{Input as PrimitiveInput, InputState},
    },
    theme::{ThemeExt, ThemeLayerKind},
    utils::{
        ElementIdExt, PixelsExt, PositionalChildren, PositionalParentElement, RgbaExt,
        disabled_transition,
    },
};

#[derive(Default)]
struct InputStyles {
    gap: Option<DefiniteLength>,
    padding: Edges<Option<DefiniteLength>>,
    inner_padding: Edges<Option<DefiniteLength>>,
    corner_radii: Corners<Option<Pixels>>,
    text_size: Option<AbsoluteLength>,
}

#[derive(IntoElement)]
pub struct Input {
    id: ElementId,
    invalid: bool,
    disabled: bool,
    layer: ThemeLayerKind,
    children: PositionalChildren,
    style: InputStyles,
    base: PrimitiveInput,
}

impl Input {
    pub fn new(id: impl Into<ElementId>, state: Entity<InputState>) -> Self {
        Self {
            id: id.into(),
            invalid: false,
            disabled: false,
            layer: ThemeLayerKind::Tertiary,
            children: PositionalChildren::default(),
            style: InputStyles::default(),
            base: PrimitiveInput::new(state),
        }
    }

    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn placeholder_text_color(mut self, color: impl Into<Hsla>) -> Self {
        self.base = self.base.placeholder_text_color(color);
        self
    }

    pub fn selection_color(mut self, color: impl Into<Hsla>) -> Self {
        self.base = self.base.selection_color(color);
        self
    }

    pub fn placeholder(mut self, text: impl Into<SharedString>) -> Self {
        self.base = self.base.placeholder(text);
        self
    }

    pub fn initial_value(mut self, text: impl Into<SharedString>, cx: &mut App) -> Self {
        self.base = self.base.initial_value(text, cx);
        self
    }

    pub fn read_text(&self, cx: &mut App) -> SharedString {
        self.base.read_text(cx)
    }

    pub fn rounded(mut self, rounded: impl Into<Pixels>) -> Self {
        let rounded = rounded.into();
        self.style.corner_radii = Corners::all(Some(rounded));
        self
    }

    pub fn rounded_tl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.top_left = Some(rounded.into());
        self
    }

    pub fn rounded_tr(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.top_right = Some(rounded.into());
        self
    }

    pub fn rounded_bl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.bottom_left = Some(rounded.into());
        self
    }

    pub fn rounded_br(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.bottom_right = Some(rounded.into());
        self
    }

    pub fn gap(mut self, gap: impl Into<DefiniteLength>) -> Self {
        self.style.gap = Some(gap.into());
        self
    }

    pub fn p(mut self, padding: impl Into<DefiniteLength>) -> Self {
        let padding = padding.into();
        self.style.padding = Edges::all(Some(padding));
        self
    }

    pub fn pt(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.top = Some(padding.into());
        self
    }

    pub fn pb(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.bottom = Some(padding.into());
        self
    }

    pub fn pl(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.left = Some(padding.into());
        self
    }

    pub fn pr(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.right = Some(padding.into());
        self
    }

    pub fn inner_p(mut self, padding: impl Into<DefiniteLength>) -> Self {
        let padding = padding.into();
        self.style.inner_padding = Edges::all(Some(padding));
        self
    }

    pub fn inner_pt(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.inner_padding.top = Some(padding.into());
        self
    }

    pub fn inner_pb(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.inner_padding.bottom = Some(padding.into());
        self
    }

    pub fn inner_pl(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.inner_padding.left = Some(padding.into());
        self
    }

    pub fn inner_pr(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.inner_padding.right = Some(padding.into());
        self
    }

    pub fn text_size(mut self, padding: impl Into<AbsoluteLength>) -> Self {
        self.style.text_size = Some(padding.into());
        self
    }
}

macro_rules! apply_corner_radii {
    ($this:expr, $corner_radii_override:expr, $corner_radius:expr) => {
        $this
            .rounded_tl(
                $corner_radii_override
                    .top_left
                    .unwrap_or($corner_radius.into()),
            )
            .rounded_tr(
                $corner_radii_override
                    .top_right
                    .unwrap_or($corner_radius.into()),
            )
            .rounded_bl(
                $corner_radii_override
                    .bottom_left
                    .unwrap_or($corner_radius.into()),
            )
            .rounded_br(
                $corner_radii_override
                    .bottom_right
                    .unwrap_or($corner_radius.into()),
            )
    };
}

macro_rules! apply_padding {
    (
        $this:expr,
        $padding_override:expr,
        $vertical_padding:expr,
        $horizontal_padding:expr
    ) => {
        $this
            .pt($padding_override.top.unwrap_or($vertical_padding.into()))
            .pb($padding_override.bottom.unwrap_or($vertical_padding.into()))
            .pl($padding_override.left.unwrap_or($horizontal_padding.into()))
            .pr($padding_override
                .right
                .unwrap_or($horizontal_padding.into()))
    };
}

impl RenderOnce for Input {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let (primary_text_color, secondary_text_color) =
            cx.get_theme().variants.active(cx).colors.text.all();
        let primary_accent_color = cx.get_theme().variants.active(cx).colors.accent.primary;
        let destructive_accent_color = cx.get_theme().variants.active(cx).colors.accent.destructive;
        let background_color = self.layer.resolve(cx);
        let border_color = self.layer.next().resolve(cx);
        let border_hover_color = border_color.apply_delta(&primary_text_color, 0.07);
        let font_family = cx.get_theme().layout.text.default_font.family[0].clone();
        let line_height = cx.get_theme().layout.text.default_font.line_height;
        let text_size = self
            .style
            .text_size
            .unwrap_or_else(|| cx.get_theme().layout.text.default_font.sizes.body.clone());
        let corner_radius = cx.get_theme().layout.corner_radii.md;
        let corner_radii_override = self.style.corner_radii;
        let padding_override = self.style.padding;
        let inner_padding_override = self.style.inner_padding;
        let horizontal_padding = cx.get_theme().layout.padding.lg;
        let vertical_padding =
            cx.get_theme()
                .layout
                .size
                .lg
                .padding_needed_for_height(window, text_size, line_height);

        let is_invalid = self.invalid;

        let is_hover_state =
            window.use_keyed_state(self.id.with_suffix("state:hover"), cx, |_cx, _window| false);
        let is_hover = *is_hover_state.read(cx);

        let focus_handle = self.focus_handle(cx).clone();
        let is_focus = focus_handle.is_focused(window);

        let is_disabled = self.disabled;
        let disabled_transition_state =
            disabled_transition(self.id.clone(), window, cx, is_disabled);

        if is_focus && is_disabled {
            window.blur();
        }

        let border_color_transition_state = conitional_transition!(
            self.id.with_suffix("state:transition:border_color"),
            window,
            cx,
            Duration::from_millis(400),
            {
                is_invalid => destructive_accent_color,
                is_focus => primary_accent_color,
                is_hover => border_hover_color,
                _ => border_color
            }
        )
        .with_easing(ease_out_quint());

        let focus_ring_color_transition_state = conitional_transition!(
            self.id.with_suffix("state:transition:focus_ring_color"),
            window,
            cx,
            Duration::from_millis(400),
            {
                is_invalid => destructive_accent_color,
                _ => primary_accent_color
            }
        )
        .with_easing(ease_out_quint());

        div()
            .id(self.id.clone())
            .w_full()
            .h_auto()
            .map(|this| {
                apply_padding!(this, padding_override, vertical_padding, horizontal_padding)
            })
            .gap(self.style.gap.unwrap_or(horizontal_padding.into()))
            .flex()
            .flex_col()
            .with_transitions(
                (disabled_transition_state, focus_ring_color_transition_state),
                move |_cx, this, (opacity, color)| {
                    this.opacity(opacity).child(
                        FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone())
                            .border_color(color)
                            .map(|this| {
                                apply_corner_radii!(this, corner_radii_override, corner_radius)
                            }),
                    )
                },
            )
            .child(
                squircle()
                    .absolute_expand()
                    .map(|this| apply_corner_radii!(this, corner_radii_override, corner_radius))
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
                    .map(|this| apply_padding!(this, inner_padding_override, px(0.), px(0.)))
                    .children(self.children.left)
                    .child(
                        self.base
                            .w_full()
                            .text_size(text_size)
                            .font_family(font_family)
                            .text_color(primary_text_color)
                            .placeholder_text_color(secondary_text_color)
                            .selection_color(primary_accent_color.alpha(0.3))
                            .line_height(line_height)
                            .disabled(is_disabled),
                    )
                    .children(self.children.right),
            )
            .children(self.children.bottom)
            .when(!is_disabled, |this| {
                this.on_hover(move |hover, _window, cx| {
                    is_hover_state.update(cx, |this, _cx| *this = *hover);
                    cx.notify(is_hover_state.entity_id());
                })
            })
    }
}

impl PositionalParentElement for Input {
    fn children_mut(&mut self) -> &mut crate::utils::PositionalChildren {
        &mut self.children
    }
}

impl Focusable for Input {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.base.focus_handle(cx)
    }
}
