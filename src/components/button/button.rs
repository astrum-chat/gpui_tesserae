use std::time::Duration;

use gpui::{
    App, ClickEvent, CursorStyle, DefiniteLength, Edges, ElementId, InteractiveElement,
    IntoElement, JustifyContent, Length, ParentElement, RenderOnce, Rgba, SharedString,
    SizeRefinement, StatefulInteractiveElement, Styled, Window, div, ease_out_quint,
    prelude::FluentBuilder, px, relative,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_tesserae_theme::ThemeExt;
use gpui_transitions::{TransitionExt, TransitionGoal};

use crate::{
    components::Icon,
    conitional_transition,
    primitives::{FocusRing, min_w0_wrapper},
    utils::{
        ElementIdExt, PixelsExt, PositionalChildren, PositionalParentElement, RgbaExt, SquircleExt,
        disabled_transition,
    },
};

struct ButtonStyles {
    justify_content: JustifyContent,
    padding: Edges<Option<DefiniteLength>>,
    width: Length,
}

impl Default for ButtonStyles {
    fn default() -> Self {
        Self {
            justify_content: JustifyContent::Center,
            padding: Edges::default(),
            width: Length::Auto,
        }
    }
}

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    text: Option<SharedString>,
    icon: Option<SharedString>,
    icon_size: SizeRefinement<Length>,
    variant: ButtonVariantEither,
    disabled: bool,
    on_hover: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    on_click: Option<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    children: PositionalChildren,
    style: ButtonStyles,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            text: None,
            icon: None,
            icon_size: SizeRefinement {
                width: Some(px(0.).into()),
                height: Some(px(0.).into()),
            },
            variant: ButtonVariantEither::Left(ButtonVariant::Primary),
            disabled: false,
            on_hover: None,
            on_click: None,
            children: PositionalChildren::default(),
            style: ButtonStyles::default(),
        }
    }

    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn no_text(mut self) -> Self {
        self.text = None;
        self
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn icon_size(mut self, icon_size: impl Into<Length>) -> Self {
        let icon_size = icon_size.into();
        self.icon_size = SizeRefinement {
            width: Some(icon_size),
            height: Some(icon_size),
        };
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

    /// Sets the element to justify flex items against the start of the container's main axis.
    /// [Docs](https://tailwindcss.com/docs/justify-content#start)
    pub fn justify_start(mut self) -> Self {
        self.style.justify_content = JustifyContent::Start;
        self
    }

    /// Sets the element to justify flex items against the end of the container's main axis.
    /// [Docs](https://tailwindcss.com/docs/justify-content#end)
    pub fn justify_end(mut self) -> Self {
        self.style.justify_content = JustifyContent::End;
        self
    }

    /// Sets the element to justify flex items along the center of the container's main axis.
    /// [Docs](https://tailwindcss.com/docs/justify-content#center)
    pub fn justify_center(mut self) -> Self {
        self.style.justify_content = JustifyContent::Center;
        self
    }

    /// Sets the element to justify flex items along the container's main axis
    /// such that there is an equal amount of space between each item.
    /// [Docs](https://tailwindcss.com/docs/justify-content#space-between)
    pub fn justify_between(mut self) -> Self {
        self.style.justify_content = JustifyContent::SpaceBetween;
        self
    }

    /// Sets the element to justify items along the container's main axis such
    /// that there is an equal amount of space on each side of each item.
    /// [Docs](https://tailwindcss.com/docs/justify-content#space-around)
    pub fn justify_around(mut self) -> Self {
        self.style.justify_content = JustifyContent::SpaceAround;
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

    pub fn w(mut self, width: impl Into<Length>) -> Self {
        self.style.width = width.into();
        self
    }

    pub fn w_auto(mut self) -> Self {
        self.style.width = Length::Auto;
        self
    }

    pub fn w_full(mut self) -> Self {
        self.style.width = relative(100.).into();
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

impl RenderOnce for Button {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let variant = self.variant.into_granular(cx);
        let font_family = cx.get_theme().layout.text.default_font.family[0].clone();
        let line_height = cx.get_theme().layout.text.default_font.line_height;
        let text_size = cx.get_theme().layout.text.default_font.sizes.body.clone();
        let padding_override = self.style.padding;
        let corner_radius = cx.get_theme().layout.corner_radii.md;
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
            Duration::from_millis(250),
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
            Duration::from_millis(250),
            variant.text_color
        )
        .with_easing(ease_out_quint());

        let highlight_alpha_state = conitional_transition!(
            self.id.with_suffix("state:transition:highlight_alpha"),
            window,
            cx,
            Duration::from_millis(250),
            {
                is_focus || is_click_down || is_hover => variant.highlight_active_alpha,
                _ => variant.highlight_alpha
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
            .w(self.style.width)
            .h_auto()
            .map(|this| {
                apply_padding!(this, padding_override, vertical_padding, horizontal_padding)
            })
            .gap(horizontal_padding)
            .flex()
            .flex_col()
            .with_transitions(disabled_transition_state, |_cx, this, opacity| {
                this.opacity(opacity)
            })
            .child(
                FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone())
                    .rounded(corner_radius.clone()),
            )
            .child(
                squircle()
                    .absolute_expand()
                    .rounded(corner_radius)
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
                    .gap(horizontal_padding)
                    .map(|mut this| {
                        this.style().justify_content = Some(self.style.justify_content);
                        this
                    })
                    .items_center()
                    .children(self.children.left)
                    .with_transitions(text_color_state, move |_cx, this, text_color| {
                        this.text_color(text_color)
                            .when_some(self.icon.as_ref(), |this, icon| {
                                this.child(Icon::new(icon).color(text_color).map(|mut this| {
                                    this.size = self.icon_size.clone();
                                    this
                                }))
                            })
                    })
                    .when_some(self.text, |this, text| {
                        this.child(
                            min_w0_wrapper()
                                .font_family(font_family.clone())
                                .text_size(text_size)
                                .text_ellipsis()
                                .child(text),
                        )
                    })
                    .children(self.children.right),
            )
            .children(self.children.bottom)
            .when(!self.disabled, |this| {
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
                    move |event, window, cx| {
                        window.prevent_default();

                        if !is_focus {
                            // We only want to blur if something else may be focused.
                            window.blur();
                        }

                        is_click_down_state_on_click.update(cx, |this, _cx| *this = false);
                        cx.notify(is_click_down_state_on_click.entity_id());

                        Self::handle_on_click(window, cx, event, self.on_click.as_ref());
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

impl PositionalParentElement for Button {
    fn children_mut(&mut self) -> &mut crate::utils::PositionalChildren {
        &mut self.children
    }
}

#[derive(Clone)]
pub struct GranularButtonVariant {
    pub bg_color: Rgba,
    pub bg_hover_color: Rgba,
    pub bg_focus_color: Rgba,
    pub text_color: Rgba,
    pub highlight_alpha: f32,
    pub highlight_active_alpha: f32,
}

pub enum ButtonVariant {
    Primary,
    Secondary,
    SecondaryGhost,
    Constructive,
    ConstructiveGhost,
    Destructive,
    DestructiveGhost,
}

impl ButtonVariant {
    pub fn as_granular(&self, cx: &mut App) -> GranularButtonVariant {
        const HOVER_STRENGTH: f32 = 0.15;
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
                highlight_active_alpha: 0.05,
            }
        }

        fn ghost_variant(primary_background: &Rgba, main_color: &Rgba) -> GranularButtonVariant {
            GranularButtonVariant {
                bg_color: main_color.alpha(0.),
                bg_hover_color: main_color.alpha(SECONDARY_ALPHA),
                bg_focus_color: main_color
                    .apply_delta(&primary_background, HOVER_STRENGTH)
                    .alpha(SECONDARY_ALPHA),
                text_color: *main_color,
                highlight_alpha: 0.,
                highlight_active_alpha: 0.05,
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
                highlight_active_alpha: 0.15,
            },

            ButtonVariant::Secondary => {
                secondary_variant(&primary_background, &colors.text.secondary)
            }

            ButtonVariant::SecondaryGhost => {
                ghost_variant(&primary_background, &colors.text.secondary)
            }

            ButtonVariant::Constructive => {
                secondary_variant(&primary_background, &colors.accent.constructive)
            }

            ButtonVariant::ConstructiveGhost => {
                ghost_variant(&primary_background, &colors.accent.constructive)
            }

            ButtonVariant::Destructive => {
                secondary_variant(&primary_background, &colors.accent.destructive)
            }

            ButtonVariant::DestructiveGhost => {
                ghost_variant(&primary_background, &colors.accent.destructive)
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
