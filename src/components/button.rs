use std::time::Duration;

use gpui::{
    App, ClickEvent, CursorStyle, ElementId, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, Rgba, SharedString, StatefulInteractiveElement, Styled, Window, div,
    ease_out_quint, prelude::FluentBuilder, px,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_tesserae_theme::ThemeExt;
use gpui_transitions::{TransitionExt, TransitionGoal};

use crate::{
    conitional_transition,
    primitives::FocusRing,
    utils::{
        ElementIdExt, PixelsExt, PositionalChildren, PositionalParentElement, RgbaExt, SquircleExt,
        disabled_transition,
    },
};

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    text: SharedString,
    variant: ButtonVariantEither,
    disabled: bool,
    on_hover: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    children: PositionalChildren,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            text: SharedString::from("Button"),
            variant: ButtonVariantEither::Left(ButtonVariant::Primary),
            disabled: false,
            on_hover: None,
            on_click: None,
            children: PositionalChildren::default(),
        }
    }

    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.text = text.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_hover(mut self, on_hover: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Box::new(on_hover));
        self
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(on_click));
        self
    }

    // ButtonVariantEither is an internal wrapper type for
    // allowing both `ButtonVariant` and `GranularButtonVariant`.
    // It does not need to be public.
    #[allow(private_bounds)]
    pub fn variant(mut self, variant: impl Into<ButtonVariantEither>) -> Self {
        self.variant = variant.into();
        self
    }

    fn handle_on_click(
        window: &mut Window,
        cx: &mut App,
        event: &ClickEvent,
        on_click: Option<&Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    ) {
        if let Some(on_click) = on_click {
            (on_click)(event, window, cx)
        }
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let variant = self.variant.into_granular(cx);

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

        let is_disabled = self.disabled;
        let disabled_transition_state =
            disabled_transition(self.id.clone(), window, cx, is_disabled);

        if is_focus && is_disabled {
            window.blur();
        }

        let bg_color_state = conitional_transition!(
            self.id.with_suffix("state:transition:bg_color"),
            window,
            cx,
            Duration::from_millis(365),
            {
                is_focus || is_click_down => variant.bg_focus_color,
                is_hover => variant.bg_hover_color,
                _ => variant.bg_color
            }
        )
        .with_easing(ease_out_quint());

        let text_color_state = conitional_transition!(
            self.id.with_suffix("state:transition:text_color"),
            window,
            cx,
            Duration::from_millis(365),
            variant.text_color
        )
        .with_easing(ease_out_quint());

        let highlight_alpha_state = conitional_transition!(
            self.id.with_suffix("state:transition:highlight_alpha"),
            window,
            cx,
            Duration::from_millis(365),
            variant.highlight_alpha
        )
        .with_easing(ease_out_quint());

        div()
            .id(self.id.clone())
            .cursor(if is_disabled {
                CursorStyle::OperationNotAllowed
            } else {
                CursorStyle::PointingHand
            })
            .w_full()
            .h_auto()
            .pl(horizontal_padding)
            .pr(horizontal_padding)
            .pt(vertical_padding)
            .pb(vertical_padding)
            .gap(horizontal_padding)
            .flex()
            .flex_col()
            .with_transitions(disabled_transition_state, |_cx, this, opacity| {
                this.opacity(opacity)
            })
            .child(
                FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone())
                    .rounded(corner_radii.clone()),
            )
            .child(
                squircle()
                    .absolute_expand()
                    .rounded(corner_radii)
                    .border(px(1.))
                    .border_inside()
                    .with_transitions(
                        (bg_color_state, highlight_alpha_state),
                        move |_cx, this, (bg_color, highlight_alpha)| {
                            this.bg(bg_color).border_highlight_color(highlight_alpha)
                        },
                    ),
            )
            .children(self.children.top)
            .child(
                div()
                    .w_full()
                    .flex()
                    .justify_center()
                    .gap(horizontal_padding)
                    .items_center()
                    .font_family(font_family.clone())
                    .text_size(text_size)
                    .children(self.children.left)
                    .child(self.text)
                    .children(self.children.right)
                    .with_transitions(text_color_state, |_cx, this, text_color| {
                        this.text_color(text_color)
                    }),
            )
            .children(self.children.bottom)
            .when(!self.disabled, |this| {
                this.on_hover(move |hover, _window, cx| {
                    is_hover_state.update(cx, |this, _cx| *this = *hover);
                    cx.notify(is_hover_state.entity_id());
                })
                .map(|this| {
                    let is_click_down_state = is_click_down_state.clone();

                    this.on_mouse_down(gpui::MouseButton::Left, move |_, window, cx| {
                        // Prevents focus ring from appearing when clicked.
                        window.prevent_default();

                        is_click_down_state.update(cx, |this, _cx| *this = true);
                        cx.notify(is_click_down_state.entity_id());
                    })
                })
                .on_click({
                    move |event, window, cx| {
                        window.prevent_default();

                        is_click_down_state.update(cx, |this, _cx| *this = false);
                        cx.notify(is_click_down_state.entity_id());

                        Self::handle_on_click(window, cx, event, self.on_click.as_ref());
                    }
                })
                .track_focus(&focus_handle)
            })
    }
}

impl PositionalParentElement for Button {
    fn children_mut(&mut self) -> &mut crate::utils::PositionalChildren {
        &mut self.children
    }
}

pub struct GranularButtonVariant {
    pub bg_color: Rgba,
    pub bg_hover_color: Rgba,
    pub bg_focus_color: Rgba,
    pub text_color: Rgba,
    pub highlight_alpha: f32,
}

pub enum ButtonVariant {
    Primary,
    Secondary,
    Constructive,
    Destructive,
}

impl ButtonVariant {
    fn as_granular(&self, cx: &mut App) -> GranularButtonVariant {
        const HOVER_STRENGTH: f32 = 0.25;
        const FOCUS_STRENGTH: f32 = 0.35;

        const SECONDARY_ALPHA: f32 = 0.1;

        let colors = &cx.get_theme().variants.active().colors;
        let primary_background = colors.background.primary;

        fn secondary_variant(
            primary_background: &Rgba,
            main_color: &Rgba,
        ) -> GranularButtonVariant {
            GranularButtonVariant {
                bg_color: main_color.alpha(SECONDARY_ALPHA),
                bg_hover_color: main_color
                    .apply_delta(&primary_background, HOVER_STRENGTH)
                    .alpha(SECONDARY_ALPHA),
                bg_focus_color: main_color
                    .apply_delta(&primary_background, FOCUS_STRENGTH)
                    .alpha(SECONDARY_ALPHA),
                text_color: *main_color,
                highlight_alpha: 0.05,
            }
        }

        match self {
            ButtonVariant::Primary => GranularButtonVariant {
                bg_color: colors.accent.primary,
                bg_hover_color: colors
                    .accent
                    .primary
                    .apply_delta(&primary_background, HOVER_STRENGTH),
                bg_focus_color: colors
                    .accent
                    .primary
                    .apply_delta(&primary_background, FOCUS_STRENGTH),
                text_color: colors.text.primary,
                highlight_alpha: 0.15,
            },

            ButtonVariant::Secondary => {
                secondary_variant(&primary_background, &colors.text.secondary)
            }

            ButtonVariant::Constructive => {
                secondary_variant(&primary_background, &colors.accent.constructive)
            }

            ButtonVariant::Destructive => {
                secondary_variant(&primary_background, &colors.accent.destructive)
            }
        }
    }
}

enum ButtonVariantEither {
    Left(ButtonVariant),
    Right(GranularButtonVariant),
}

impl ButtonVariantEither {
    fn into_granular(self, cx: &mut App) -> GranularButtonVariant {
        match self {
            ButtonVariantEither::Left(left) => left.as_granular(cx),
            ButtonVariantEither::Right(right) => right,
        }
    }
}

impl From<ButtonVariant> for ButtonVariantEither {
    fn from(value: ButtonVariant) -> Self {
        ButtonVariantEither::Left(value)
    }
}

impl From<GranularButtonVariant> for ButtonVariantEither {
    fn from(value: GranularButtonVariant) -> Self {
        ButtonVariantEither::Right(value)
    }
}
