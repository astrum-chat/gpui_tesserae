use std::time::Duration;

use gpui::{
    App, Corners, CursorStyle, DefiniteLength, Edges, ElementId, FocusHandle, InteractiveElement,
    IntoElement, JustifyContent, Length, ParentElement, Pixels, Radians, RenderOnce, Rgba,
    SharedString, SizeRefinement, StatefulInteractiveElement, Styled, Window, div, ease_out_quint,
    prelude::FluentBuilder, px, relative,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::Lerp;

use crate::{
    components::Icon,
    conitional_transition,
    extensions::{
        click_behavior::{ClickBehavior, ClickBehaviorExt},
        clickable::{ClickHandlers, Clickable},
    },
    primitives::{FocusRing, min_w0_wrapper},
    theme::ThemeExt,
    utils::{
        ElementIdExt, PixelsExt, PositionalChildren, PositionalParentElement, RgbaExt, SquircleExt,
        disabled_transition,
    },
};

struct ButtonStyles {
    justify_content: JustifyContent,
    padding: Edges<Option<DefiniteLength>>,
    corner_radii: Corners<Option<Pixels>>,
    icon_rotate: Radians,
    width: Length,
    min_width: Option<Length>,
    min_height: Option<Length>,
    max_width: Option<Length>,
    max_height: Option<Length>,
}

impl Default for ButtonStyles {
    fn default() -> Self {
        Self {
            justify_content: JustifyContent::Center,
            padding: Edges::default(),
            corner_radii: Corners::default(),
            icon_rotate: Radians(0.),
            width: Length::Auto,
            min_width: None,
            min_height: None,
            max_width: None,
            max_height: None,
        }
    }
}

/// A themed button component with multiple visual variants.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    text: Option<SharedString>,
    icon: Option<SharedString>,
    icon_size: SizeRefinement<Length>,
    variant: ButtonVariantEither,
    disabled: bool,
    force_hover: bool,
    focus_handle: Option<FocusHandle>,
    on_hover: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    click_handlers: ClickHandlers,
    click_behavior: ClickBehavior,
    children: PositionalChildren,
    style: ButtonStyles,
}

impl Button {
    /// Creates a new button with the given element ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            text: None,
            icon: None,
            icon_size: SizeRefinement {
                width: Some(px(14.).into()),
                height: Some(px(14.).into()),
            },
            variant: ButtonVariantEither::Left(ButtonVariant::Primary),
            disabled: false,
            force_hover: false,
            focus_handle: None,
            on_hover: None,
            click_handlers: ClickHandlers::new(),
            click_behavior: ClickBehavior::default(),
            children: PositionalChildren::default(),
            style: ButtonStyles::default(),
        }
    }

    /// Sets the focus handle for keyboard navigation.
    pub fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle);
        self
    }

    /// Sets the button's text label.
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Removes any text label from the button.
    pub fn no_text(mut self) -> Self {
        self.text = None;
        self
    }

    /// Sets an icon to display in the button.
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Sets uniform width and height for the icon.
    pub fn icon_size(mut self, icon_size: impl Into<Length>) -> Self {
        let icon_size = icon_size.into();
        self.icon_size = SizeRefinement {
            width: Some(icon_size),
            height: Some(icon_size),
        };
        self
    }

    /// Applies a rotation transformation to the icon.
    pub fn icon_rotate(mut self, rotate: impl Into<Radians>) -> Self {
        self.style.icon_rotate = rotate.into();
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
    pub fn on_hover(mut self, on_hover: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_hover = Some(Box::new(on_hover));
        self
    }

    /// Sets the visual variant determining colors and styling.
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
        self.style.justify_content = JustifyContent::FlexStart;
        self
    }

    /// Sets the element to justify flex items against the end of the container's main axis.
    /// [Docs](https://tailwindcss.com/docs/justify-content#end)
    pub fn justify_end(mut self) -> Self {
        self.style.justify_content = JustifyContent::FlexEnd;
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

    /// Sets uniform corner radius for all corners.
    pub fn rounded(mut self, rounded: impl Into<Pixels>) -> Self {
        let rounded = rounded.into();
        self.style.corner_radii = Corners::all(Some(rounded));
        self
    }

    /// Sets the top-left corner radius.
    pub fn rounded_tl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.top_left = Some(rounded.into());
        self
    }

    /// Sets the top-right corner radius.
    pub fn rounded_tr(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.top_right = Some(rounded.into());
        self
    }

    /// Sets the bottom-left corner radius.
    pub fn rounded_bl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.bottom_left = Some(rounded.into());
        self
    }

    /// Sets the bottom-right corner radius.
    pub fn rounded_br(mut self, rounded: impl Into<Pixels>) -> Self {
        self.style.corner_radii.bottom_right = Some(rounded.into());
        self
    }

    /// Sets uniform padding for all sides.
    pub fn p(mut self, padding: impl Into<DefiniteLength>) -> Self {
        let padding = padding.into();
        self.style.padding = Edges::all(Some(padding));
        self
    }

    /// Sets top padding.
    pub fn pt(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.top = Some(padding.into());
        self
    }

    /// Sets bottom padding.
    pub fn pb(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.bottom = Some(padding.into());
        self
    }

    /// Sets left padding.
    pub fn pl(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.left = Some(padding.into());
        self
    }

    /// Sets right padding.
    pub fn pr(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.style.padding.right = Some(padding.into());
        self
    }

    /// Sets a fixed width.
    pub fn w(mut self, width: impl Into<Length>) -> Self {
        self.style.width = width.into();
        self
    }

    /// Sets width to auto, sizing based on content.
    pub fn w_auto(mut self) -> Self {
        self.style.width = Length::Auto;
        self
    }

    /// Sets width to fill the parent container.
    pub fn w_full(mut self) -> Self {
        self.style.width = relative(100.).into();
        self
    }

    /// Sets the minimum width of the element. [Docs](https://tailwindcss.com/docs/min-width)
    pub fn min_w(mut self, width: impl Into<Length>) -> Self {
        self.style.min_width = Some(width.into());
        self
    }

    /// Sets the minimum width to 0. [Docs](https://tailwindcss.com/docs/min-width)
    pub fn min_w_0(mut self) -> Self {
        self.style.min_width = Some(px(0.).into());
        self
    }

    /// Sets the minimum width to 100%. [Docs](https://tailwindcss.com/docs/min-width)
    pub fn min_w_full(mut self) -> Self {
        self.style.min_width = Some(relative(100.).into());
        self
    }

    /// Sets the minimum height of the element. [Docs](https://tailwindcss.com/docs/min-height)
    pub fn min_h(mut self, height: impl Into<Length>) -> Self {
        self.style.min_height = Some(height.into());
        self
    }

    /// Sets the minimum height to 0. [Docs](https://tailwindcss.com/docs/min-height)
    pub fn min_h_0(mut self) -> Self {
        self.style.min_height = Some(px(0.).into());
        self
    }

    /// Sets the minimum height to 100%. [Docs](https://tailwindcss.com/docs/min-height)
    pub fn min_h_full(mut self) -> Self {
        self.style.min_height = Some(relative(100.).into());
        self
    }

    /// Sets the maximum width of the element. [Docs](https://tailwindcss.com/docs/max-width)
    pub fn max_w(mut self, width: impl Into<Length>) -> Self {
        self.style.max_width = Some(width.into());
        self
    }

    /// Sets the maximum width to 0. [Docs](https://tailwindcss.com/docs/max-width)
    pub fn max_w_0(mut self) -> Self {
        self.style.max_width = Some(px(0.).into());
        self
    }

    /// Sets the maximum width to 100%. [Docs](https://tailwindcss.com/docs/max-width)
    pub fn max_w_full(mut self) -> Self {
        self.style.max_width = Some(relative(100.).into());
        self
    }

    /// Sets the maximum height of the element. [Docs](https://tailwindcss.com/docs/max-height)
    pub fn max_h(mut self, height: impl Into<Length>) -> Self {
        self.style.max_height = Some(height.into());
        self
    }

    /// Sets the maximum height to 0. [Docs](https://tailwindcss.com/docs/max-height)
    pub fn max_h_0(mut self) -> Self {
        self.style.max_height = Some(px(0.).into());
        self
    }

    /// Sets the maximum height to 100%. [Docs](https://tailwindcss.com/docs/max-height)
    pub fn max_h_full(mut self) -> Self {
        self.style.max_height = Some(relative(100.).into());
        self
    }
}

impl Clickable for Button {
    fn click_handlers_mut(&mut self) -> &mut ClickHandlers {
        &mut self.click_handlers
    }
}

impl ClickBehaviorExt for Button {
    fn click_behavior_mut(&mut self) -> &mut ClickBehavior {
        &mut self.click_behavior
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

impl RenderOnce for Button {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let variant = self.variant.into_granular(cx);
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

        let is_disabled = self.disabled;
        let disabled_transition = disabled_transition(self.id.clone(), window, cx, is_disabled);

        if is_focus && is_disabled {
            window.blur();
        }

        let bg_color_transition = conitional_transition!(
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

        let text_color_transition = conitional_transition!(
            self.id.with_suffix("state:transition:text_color"),
            window,
            cx,
            Duration::from_millis(250),
            variant.text_color
        )
        .with_easing(ease_out_quint());

        let highlight_alpha_transition = conitional_transition!(
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
            .when_some(self.style.min_width, |this, v| this.min_w(v))
            .when_some(self.style.min_height, |this, v| this.min_h(v))
            .when_some(self.style.max_width, |this, v| this.max_w(v))
            .when_some(self.style.max_height, |this, v| this.max_h(v))
            .map(|this| {
                apply_padding!(this, padding_override, vertical_padding, horizontal_padding)
            })
            .gap(horizontal_padding)
            .flex()
            .flex_col()
            .opacity(*disabled_transition.evaluate(window, cx))
            .child(
                FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone())
                    .map(|this| apply_corner_radii!(this, self.style.corner_radii, corner_radius)),
            )
            .child(
                squircle()
                    .absolute_expand()
                    .map(|this| apply_corner_radii!(this, self.style.corner_radii, corner_radius))
                    .border(px(1.))
                    .border_inside()
                    .bg(*bg_color_transition.evaluate(window, cx))
                    .border_highlight(*highlight_alpha_transition.evaluate(window, cx)),
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
                    .text_color(*text_color_transition.evaluate(window, cx))
                    .children(self.children.left)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(horizontal_padding)
                            .map(|this| {
                                let text_color = *text_color_transition.evaluate(window, cx);

                                this.when_some(self.icon.as_ref(), |this, icon| {
                                    this.child(
                                        Icon::new(icon)
                                            .color(text_color)
                                            .rotate(self.style.icon_rotate)
                                            .map(|mut this| {
                                                this.size = self.icon_size.clone();
                                                this
                                            }),
                                    )
                                })
                                .when_some(
                                    self.text.clone(),
                                    |this, text| {
                                        this.child(
                                            min_w0_wrapper().child(text).text_color(text_color),
                                        )
                                    },
                                )
                            }),
                    )
                    .children(self.children.right),
            )
            .children(self.children.bottom)
            .when(!self.disabled, |this| {
                let is_hover_state_on_hover = is_hover_state.clone();
                let is_click_down_state_on_mouse_down = is_click_down_state.clone();
                let is_click_down_state_on_click = is_click_down_state.clone();
                let behavior = self.click_behavior;

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
                    let behavior = self.click_behavior;

                    if let Some((button, handler)) = self.click_handlers.on_mouse_down {
                        if button != gpui::MouseButton::Left {
                            this = this.on_mouse_down(button, move |event, window, cx| {
                                behavior.apply(window, cx);
                                (handler)(event, window, cx);
                            });
                        }
                    }

                    if let Some((button, handler)) = self.click_handlers.on_mouse_up {
                        this = this.on_mouse_up(button, move |event, window, cx| {
                            behavior.apply(window, cx);
                            (handler)(event, window, cx);
                        });
                    }

                    if let Some(handler) = self.click_handlers.on_any_mouse_down {
                        this = this.on_any_mouse_down(move |event, window, cx| {
                            behavior.apply(window, cx);
                            (handler)(event, window, cx);
                        });
                    }

                    if let Some(handler) = self.click_handlers.on_any_mouse_up {
                        this.interactivity()
                            .on_any_mouse_up(move |event, window, cx| {
                                behavior.apply(window, cx);
                                (handler)(event, window, cx);
                            });
                    }

                    let on_click = self.click_handlers.on_click;
                    this.on_click(move |event, window, cx| {
                        behavior.apply(window, cx);

                        if !is_focus {
                            // We only want to blur if something else may be focused.
                            window.blur();
                        }

                        is_click_down_state_on_click.update(cx, |this, cx| {
                            *this = false;
                            cx.notify();
                        });

                        if let Some(on_click) = &on_click {
                            (on_click)(event, window, cx);
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

impl PositionalParentElement for Button {
    fn children_mut(&mut self) -> &mut crate::utils::PositionalChildren {
        &mut self.children
    }
}

/// Fine-grained color configuration for button styling.
#[derive(Clone)]
pub struct GranularButtonVariant {
    /// Default background color.
    pub bg_color: Rgba,
    /// Background color when hovered.
    pub bg_hover_color: Rgba,
    /// Background color when focused.
    pub bg_focus_color: Rgba,
    /// Text and icon color.
    pub text_color: Rgba,
    /// Border highlight opacity in default state.
    pub highlight_alpha: f32,
    /// Border highlight opacity when active (hovered, focused, or clicked).
    pub highlight_active_alpha: f32,
}

/// Predefined visual styles for buttons.
pub enum ButtonVariant {
    /// Solid accent-colored button for primary actions.
    Primary,
    /// Semi-transparent button using text color.
    Secondary,
    /// Transparent button that shows background on hover.
    SecondaryGhost,
    /// Subtle button using secondary text color.
    Tertiary,
    /// Transparent subtle button.
    TertiaryGhost,
    /// Green-tinted button for positive actions.
    Constructive,
    /// Transparent green button.
    ConstructiveGhost,
    /// Red-tinted button for dangerous actions.
    Destructive,
    /// Transparent red button.
    DestructiveGhost,
}

impl ButtonVariant {
    /// Converts this variant to a granular variant using theme colors.
    pub fn as_granular(&self, cx: &App) -> GranularButtonVariant {
        const HOVER_STRENGTH: f32 = 0.15;
        const FOCUS_STRENGTH: f32 = 0.35;

        const SECONDARY_ALPHA: f32 = 0.1;

        let colors = &cx.get_theme().variants.active(cx).colors;
        let primary_background = colors.background.primary;

        fn secondary_variant(
            primary_background: &Rgba,
            main_color: &Rgba,
        ) -> GranularButtonVariant {
            GranularButtonVariant {
                bg_color: main_color.alpha(SECONDARY_ALPHA),
                bg_hover_color: main_color
                    .lerp(&primary_background, HOVER_STRENGTH)
                    .alpha(SECONDARY_ALPHA),
                bg_focus_color: main_color
                    .lerp(&primary_background, FOCUS_STRENGTH)
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
                    .lerp(&primary_background, HOVER_STRENGTH)
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
                    .lerp(&primary_background, HOVER_STRENGTH),
                bg_focus_color: colors
                    .accent
                    .primary
                    .lerp(&primary_background, FOCUS_STRENGTH),
                text_color: colors.text.primary,
                highlight_alpha: 0.15,
                highlight_active_alpha: 0.15,
            },

            ButtonVariant::Secondary => {
                secondary_variant(&primary_background, &colors.text.primary)
            }

            ButtonVariant::SecondaryGhost => {
                ghost_variant(&primary_background, &colors.text.primary)
            }

            ButtonVariant::Tertiary => {
                secondary_variant(&primary_background, &colors.text.secondary)
            }

            ButtonVariant::TertiaryGhost => {
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

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, VisualTestContext};

    #[gpui::test]
    fn test_button_creation(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button");
            assert!(!button.disabled, "Button should start enabled");
            assert!(button.text.is_none(), "Button should start with no text");
            assert!(button.icon.is_none(), "Button should start with no icon");
        });
    }

    #[gpui::test]
    fn test_button_text(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button").text("Click me");
            assert_eq!(
                button.text,
                Some("Click me".into()),
                "Button should have text"
            );

            let button = button.no_text();
            assert!(
                button.text.is_none(),
                "Button should have no text after no_text()"
            );
        });
    }

    #[gpui::test]
    fn test_button_icon(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let icon_path: SharedString = "icons/test.svg".into();
            let button = Button::new("test-button").icon(icon_path.clone());
            assert_eq!(button.icon, Some(icon_path), "Button should have icon");
        });
    }

    #[gpui::test]
    fn test_button_disabled_state(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button").disabled(true);
            assert!(button.disabled, "Button should be disabled");

            let button = Button::new("test-button").disabled(false);
            assert!(!button.disabled, "Button should be enabled");
        });
    }

    #[gpui::test]
    fn test_button_variants(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button").variant(ButtonVariant::Primary);
            assert!(
                matches!(
                    button.variant,
                    ButtonVariantEither::Left(ButtonVariant::Primary)
                ),
                "Button should have primary variant"
            );

            let button = Button::new("test-button").variant(ButtonVariant::Secondary);
            assert!(
                matches!(
                    button.variant,
                    ButtonVariantEither::Left(ButtonVariant::Secondary)
                ),
                "Button should have secondary variant"
            );

            let button = Button::new("test-button").variant(ButtonVariant::Destructive);
            assert!(
                matches!(
                    button.variant,
                    ButtonVariantEither::Left(ButtonVariant::Destructive)
                ),
                "Button should have destructive variant"
            );

            let button = Button::new("test-button").variant(ButtonVariant::Constructive);
            assert!(
                matches!(
                    button.variant,
                    ButtonVariantEither::Left(ButtonVariant::Constructive)
                ),
                "Button should have constructive variant"
            );
        });
    }

    #[gpui::test]
    fn test_button_justify_content(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button").justify_start();
            assert!(
                matches!(button.style.justify_content, JustifyContent::FlexStart),
                "Button should have justify start"
            );

            let button = Button::new("test-button").justify_end();
            assert!(
                matches!(button.style.justify_content, JustifyContent::FlexEnd),
                "Button should have justify end"
            );

            let button = Button::new("test-button").justify_center();
            assert!(
                matches!(button.style.justify_content, JustifyContent::Center),
                "Button should have justify center"
            );

            let button = Button::new("test-button").justify_between();
            assert!(
                matches!(button.style.justify_content, JustifyContent::SpaceBetween),
                "Button should have justify between"
            );

            let button = Button::new("test-button").justify_around();
            assert!(
                matches!(button.style.justify_content, JustifyContent::SpaceAround),
                "Button should have justify around"
            );
        });
    }

    #[gpui::test]
    fn test_button_width(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button").w(px(200.));
            assert!(
                matches!(button.style.width, Length::Definite(_)),
                "Button should have definite width"
            );

            let button = Button::new("test-button").w_auto();
            assert!(
                matches!(button.style.width, Length::Auto),
                "Button should have auto width"
            );

            let button = Button::new("test-button").w_full();
            assert!(
                matches!(button.style.width, Length::Definite(_)),
                "Button should have full width"
            );
        });
    }

    #[gpui::test]
    fn test_button_rounded(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button").rounded(px(8.));
            assert!(
                button.style.corner_radii.top_left.is_some(),
                "Button should have rounded corners"
            );
            assert!(
                button.style.corner_radii.top_right.is_some(),
                "Button should have rounded corners"
            );
            assert!(
                button.style.corner_radii.bottom_left.is_some(),
                "Button should have rounded corners"
            );
            assert!(
                button.style.corner_radii.bottom_right.is_some(),
                "Button should have rounded corners"
            );
        });
    }

    #[gpui::test]
    fn test_button_individual_corners(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button")
                .rounded_tl(px(4.))
                .rounded_tr(px(8.))
                .rounded_bl(px(12.))
                .rounded_br(px(16.));

            assert_eq!(button.style.corner_radii.top_left, Some(px(4.)));
            assert_eq!(button.style.corner_radii.top_right, Some(px(8.)));
            assert_eq!(button.style.corner_radii.bottom_left, Some(px(12.)));
            assert_eq!(button.style.corner_radii.bottom_right, Some(px(16.)));
        });
    }

    #[gpui::test]
    fn test_button_padding(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button").p(px(10.));
            assert!(
                button.style.padding.top.is_some(),
                "Button should have top padding"
            );
            assert!(
                button.style.padding.bottom.is_some(),
                "Button should have bottom padding"
            );
            assert!(
                button.style.padding.left.is_some(),
                "Button should have left padding"
            );
            assert!(
                button.style.padding.right.is_some(),
                "Button should have right padding"
            );
        });
    }

    #[gpui::test]
    fn test_button_individual_padding(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button")
                .pt(px(4.))
                .pb(px(8.))
                .pl(px(12.))
                .pr(px(16.));

            assert!(button.style.padding.top.is_some());
            assert!(button.style.padding.bottom.is_some());
            assert!(button.style.padding.left.is_some());
            assert!(button.style.padding.right.is_some());
        });
    }

    #[gpui::test]
    fn test_button_on_click_callback(cx: &mut TestAppContext) {
        use std::cell::Cell;
        use std::rc::Rc;

        let clicked = Rc::new(Cell::new(false));

        cx.update(|_cx| {
            let clicked_clone = clicked.clone();

            let button = Button::new("test-button").on_click(move |_event, _window, _cx| {
                clicked_clone.set(true);
            });

            assert!(
                button.click_handlers.on_click.is_some(),
                "Button should have on_click callback"
            );
        });
    }

    #[gpui::test]
    fn test_button_on_hover_callback(cx: &mut TestAppContext) {
        use std::cell::Cell;
        use std::rc::Rc;

        let hovered = Rc::new(Cell::new(false));

        cx.update(|_cx| {
            let hovered_clone = hovered.clone();

            let button = Button::new("test-button").on_hover(move |is_hover, _window, _cx| {
                hovered_clone.set(*is_hover);
            });

            assert!(
                button.on_hover.is_some(),
                "Button should have on_hover callback"
            );
        });
    }

    #[gpui::test]
    fn test_button_builder_chain(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button")
                .text("Click me")
                .icon("icons/test.svg")
                .disabled(false)
                .variant(ButtonVariant::Primary)
                .justify_center()
                .rounded(px(8.))
                .p(px(10.))
                .w(px(200.));

            assert_eq!(button.text, Some("Click me".into()));
            assert!(button.icon.is_some());
            assert!(!button.disabled);
        });
    }

    #[gpui::test]
    fn test_button_icon_rotate(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button").icon_rotate(Radians(std::f32::consts::PI));
            assert_eq!(
                button.style.icon_rotate.0,
                std::f32::consts::PI,
                "Button should have rotated icon"
            );
        });
    }

    #[gpui::test]
    fn test_button_icon_size(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let button = Button::new("test-button").icon_size(px(24.));
            assert!(
                button.icon_size.width.is_some(),
                "Button should have custom icon size"
            );
            assert!(
                button.icon_size.height.is_some(),
                "Button should have custom icon size"
            );
        });
    }

    #[gpui::test]
    fn test_button_renders_in_window(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};

        let window = cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            cx.open_window(Default::default(), |_window, cx| {
                cx.new(|_cx| ButtonTestView)
            })
            .unwrap()
        });

        let _cx = VisualTestContext::from_window(window.into(), cx);

        // The window creation itself validates rendering works
    }

    #[gpui::test]
    fn test_granular_button_variant(cx: &mut TestAppContext) {
        use gpui::rgba;

        cx.update(|_cx| {
            let granular = GranularButtonVariant {
                bg_color: rgba(0xFF0000FF).into(),
                bg_hover_color: rgba(0x00FF00FF).into(),
                bg_focus_color: rgba(0x0000FFFF).into(),
                text_color: rgba(0xFFFFFFFF).into(),
                highlight_alpha: 0.1,
                highlight_active_alpha: 0.2,
            };

            let button = Button::new("test-button").variant(granular);

            assert!(
                matches!(button.variant, ButtonVariantEither::Right(_)),
                "Button should have granular variant"
            );
        });
    }

    #[gpui::test]
    fn test_button_on_any_mouse_down_callback(cx: &mut TestAppContext) {
        use crate::extensions::clickable::Clickable;

        cx.update(|_cx| {
            let button =
                Button::new("test-button").on_any_mouse_down(move |_event, _window, _cx| {});

            assert!(
                button.click_handlers.on_any_mouse_down.is_some(),
                "Button should have on_any_mouse_down callback"
            );
        });
    }

    #[gpui::test]
    fn test_button_on_any_mouse_up_callback(cx: &mut TestAppContext) {
        use crate::extensions::clickable::Clickable;

        cx.update(|_cx| {
            let button = Button::new("test-button").on_any_mouse_up(move |_event, _window, _cx| {});

            assert!(
                button.click_handlers.on_any_mouse_up.is_some(),
                "Button should have on_any_mouse_up callback"
            );
        });
    }

    #[gpui::test]
    fn test_button_on_mouse_down_callback(cx: &mut TestAppContext) {
        use crate::extensions::clickable::Clickable;
        use gpui::MouseButton;

        cx.update(|_cx| {
            let button = Button::new("test-button")
                .on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {});

            assert!(
                button.click_handlers.on_mouse_down.is_some(),
                "Button should have on_mouse_down callback"
            );

            let (button, handler) = button.click_handlers.on_mouse_down.unwrap();
            assert_eq!(button, MouseButton::Left, "Should be left mouse button");
            drop(handler);
        });
    }

    #[gpui::test]
    fn test_button_on_mouse_up_callback(cx: &mut TestAppContext) {
        use crate::extensions::clickable::Clickable;
        use gpui::MouseButton;

        cx.update(|_cx| {
            let button = Button::new("test-button")
                .on_mouse_up(MouseButton::Right, move |_event, _window, _cx| {});

            assert!(
                button.click_handlers.on_mouse_up.is_some(),
                "Button should have on_mouse_up callback"
            );

            let (button, handler) = button.click_handlers.on_mouse_up.unwrap();
            assert_eq!(button, MouseButton::Right, "Should be right mouse button");
            drop(handler);
        });
    }

    #[gpui::test]
    fn test_button_click_behavior_default(cx: &mut TestAppContext) {
        use crate::extensions::click_behavior::ClickBehaviorExt;

        cx.update(|_cx| {
            let mut button = Button::new("test-button");
            let behavior = button.click_behavior_mut();

            assert!(
                !behavior.allow_propagation,
                "Button should not allow propagation by default"
            );
            assert!(
                !behavior.allow_default,
                "Button should not allow default by default"
            );
        });
    }

    #[gpui::test]
    fn test_button_allow_click_propagation(cx: &mut TestAppContext) {
        use crate::extensions::click_behavior::ClickBehaviorExt;

        cx.update(|_cx| {
            let mut button = Button::new("test-button").allow_click_propagation();
            let behavior = button.click_behavior_mut();

            assert!(
                behavior.allow_propagation,
                "Button should allow propagation after calling allow_click_propagation"
            );
            assert!(
                !behavior.allow_default,
                "Button should still not allow default"
            );
        });
    }

    #[gpui::test]
    fn test_button_allow_default_click_behaviour(cx: &mut TestAppContext) {
        use crate::extensions::click_behavior::ClickBehaviorExt;

        cx.update(|_cx| {
            let mut button = Button::new("test-button").allow_default_click_behaviour();
            let behavior = button.click_behavior_mut();

            assert!(
                !behavior.allow_propagation,
                "Button should still not allow propagation"
            );
            assert!(
                behavior.allow_default,
                "Button should allow default after calling allow_default_click_behaviour"
            );
        });
    }

    #[gpui::test]
    fn test_button_click_behavior_chain(cx: &mut TestAppContext) {
        use crate::extensions::click_behavior::ClickBehaviorExt;

        cx.update(|_cx| {
            let mut button = Button::new("test-button")
                .allow_click_propagation()
                .allow_default_click_behaviour();
            let behavior = button.click_behavior_mut();

            assert!(
                behavior.allow_propagation,
                "Button should allow propagation"
            );
            assert!(behavior.allow_default, "Button should allow default");
        });
    }

    /// Test view that contains a Button
    struct ButtonTestView;

    impl gpui::Render for ButtonTestView {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            div()
                .size_full()
                .child(Button::new("test-button").text("Click me"))
        }
    }
}
