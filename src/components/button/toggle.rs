use gpui::{
    App, DefiniteLength, ElementId, FocusHandle, IntoElement, Length, Pixels, Radians, RenderOnce,
    SharedString, Window,
};

use crate::{
    PositionalParentElement,
    components::{Button, ButtonVariant, GranularButtonVariant},
    extensions::{
        click_behavior::{ClickBehavior, ClickBehaviorExt},
        clickable::{ClickHandlers, Clickable},
    },
    utils::RgbaExt,
};

/// A toggle button that changes appearance based on checked state.
#[derive(IntoElement)]
pub struct Toggle {
    variant: ToggleVariantEither,
    checked: bool,
    base: Button,
}

impl Toggle {
    /// Creates a new toggle button with the given element ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            checked: false,
            variant: ToggleVariantEither::Left(ToggleVariant::Primary),
            base: Button::new(id),
        }
    }

    /// Sets the button's text label.
    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.base = self.base.text(text);
        self
    }

    /// Removes any text label from the button.
    pub fn no_text(mut self) -> Self {
        self.base = self.base.no_text();
        self
    }

    /// Sets an icon to display in the button.
    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.base = self.base.icon(icon);
        self
    }

    /// Sets uniform width and height for the icon.
    pub fn icon_size(mut self, icon_size: impl Into<Length>) -> Self {
        self.base = self.base.icon_size(icon_size);
        self
    }

    /// Applies a rotation transformation to the icon.
    pub fn icon_rotation(mut self, rotate: impl Into<Radians>) -> Self {
        self.base = self.base.icon_rotate(rotate);
        self
    }

    /// Sets the checked state, which determines the visual variant used.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Sets the disabled state, preventing interaction.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.base = self.base.disabled(disabled);
        self
    }

    /// Forces the hover visual state regardless of actual hover.
    pub fn force_hover(mut self, force_hover: bool) -> Self {
        self.base = self.base.force_hover(force_hover);
        self
    }

    /// Sets a callback invoked when hover state changes.
    pub fn on_hover(mut self, on_hover: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.base = self.base.on_hover(on_hover);
        self
    }

    /// Sets the focus handle for keyboard navigation.
    pub fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.base = self.base.focus_handle(focus_handle);
        self
    }

    /// Sets the visual variant determining colors for checked and unchecked states.
    // ToggleVariantEither is an internal wrapper type for
    // allowing both `ButtonVariant` and `GranularToggleVariant`.
    // It does not need to be public.
    #[allow(private_bounds)]
    pub fn variant(mut self, variant: impl Into<ToggleVariantEither>) -> Self {
        self.variant = variant.into();
        self
    }

    /// Sets the element to justify flex items against the start of the container's main axis.
    /// [Docs](https://tailwindcss.com/docs/justify-content#start)
    pub fn justify_start(mut self) -> Self {
        self.base = self.base.justify_start();
        self
    }

    /// Sets the element to justify flex items against the end of the container's main axis.
    /// [Docs](https://tailwindcss.com/docs/justify-content#end)
    pub fn justify_end(mut self) -> Self {
        self.base = self.base.justify_end();
        self
    }

    /// Sets the element to justify flex items along the center of the container's main axis.
    /// [Docs](https://tailwindcss.com/docs/justify-content#center)
    pub fn justify_center(mut self) -> Self {
        self.base = self.base.justify_center();
        self
    }

    /// Sets the element to justify flex items along the container's main axis
    /// such that there is an equal amount of space between each item.
    /// [Docs](https://tailwindcss.com/docs/justify-content#space-between)
    pub fn justify_between(mut self) -> Self {
        self.base = self.base.justify_between();
        self
    }

    /// Sets the element to justify items along the container's main axis such
    /// that there is an equal amount of space on each side of each item.
    /// [Docs](https://tailwindcss.com/docs/justify-content#space-around)
    pub fn justify_around(mut self) -> Self {
        self.base = self.base.justify_around();
        self
    }

    /// Sets uniform corner radius for all corners.
    pub fn rounded(mut self, rounded: impl Into<Pixels>) -> Self {
        self.base = self.base.rounded(rounded);
        self
    }

    /// Sets the top-left corner radius.
    pub fn rounded_tl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.base = self.base.rounded_tl(rounded);
        self
    }

    /// Sets the top-right corner radius.
    pub fn rounded_tr(mut self, rounded: impl Into<Pixels>) -> Self {
        self.base = self.base.rounded_tr(rounded);
        self
    }

    /// Sets the bottom-left corner radius.
    pub fn rounded_bl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.base = self.base.rounded_bl(rounded);
        self
    }

    /// Sets the bottom-right corner radius.
    pub fn rounded_br(mut self, rounded: impl Into<Pixels>) -> Self {
        self.base = self.base.rounded_br(rounded);
        self
    }

    /// Sets uniform padding for all sides.
    pub fn p(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.p(padding);
        self
    }

    /// Sets top padding.
    pub fn pt(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.pt(padding);
        self
    }

    /// Sets bottom padding.
    pub fn pb(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.pb(padding);
        self
    }

    /// Sets left padding.
    pub fn pl(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.pl(padding);
        self
    }

    /// Sets right padding.
    pub fn pr(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.pr(padding);
        self
    }

    /// Sets a fixed width.
    pub fn w(mut self, width: impl Into<Length>) -> Self {
        self.base = self.base.w(width);
        self
    }

    /// Sets width to auto, sizing based on content.
    pub fn w_auto(mut self) -> Self {
        self.base = self.base.w_auto();
        self
    }

    /// Sets width to fill the parent container.
    pub fn w_full(mut self) -> Self {
        self.base = self.base.w_full();
        self
    }
}

impl Clickable for Toggle {
    fn click_handlers_mut(&mut self) -> &mut ClickHandlers {
        self.base.click_handlers_mut()
    }
}

impl ClickBehaviorExt for Toggle {
    fn click_behavior_mut(&mut self) -> &mut ClickBehavior {
        self.base.click_behavior_mut()
    }
}

impl RenderOnce for Toggle {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let variant = self.variant.into_granular(cx);

        self.base.variant(if self.checked {
            variant.truthy
        } else {
            variant.falsey
        })
    }
}

impl PositionalParentElement for Toggle {
    fn children_mut(&mut self) -> &mut crate::utils::PositionalChildren {
        self.base.children_mut()
    }
}

/// Predefined visual styles for toggle buttons.
#[derive(Clone, Copy)]
pub enum ToggleVariant {
    /// Solid accent-colored when checked.
    Primary,
    /// Semi-transparent text color styling.
    Secondary,
    /// Subtle secondary text color styling.
    Tertiary,
    /// Green-tinted styling.
    Constructive,
    /// Red-tinted styling.
    Destructive,
}

/// Fine-grained color configuration for toggle button states.
pub struct GranularToggleVariant {
    truthy: GranularButtonVariant,
    falsey: GranularButtonVariant,
}

impl GranularToggleVariant {
    fn from_button_variant(variant: ButtonVariant, cx: &mut App) -> Self {
        let variant = variant.as_granular(cx);

        Self {
            truthy: variant.clone(),
            falsey: falsey_granular_variant(variant),
        }
    }
}

enum ToggleVariantEither {
    Left(ToggleVariant),
    Right(GranularToggleVariant),
}

impl ToggleVariantEither {
    fn into_granular(self, cx: &mut App) -> GranularToggleVariant {
        match self {
            ToggleVariantEither::Left(left) => left.as_granular_toggle(cx),
            ToggleVariantEither::Right(right) => right,
        }
    }
}

impl From<ToggleVariant> for ToggleVariantEither {
    fn from(value: ToggleVariant) -> Self {
        ToggleVariantEither::Left(value)
    }
}

impl From<GranularToggleVariant> for ToggleVariantEither {
    fn from(value: GranularToggleVariant) -> Self {
        ToggleVariantEither::Right(value)
    }
}

impl ToggleVariant {
    fn as_granular_toggle(&self, cx: &mut App) -> GranularToggleVariant {
        match self {
            ToggleVariant::Primary => {
                GranularToggleVariant::from_button_variant(ButtonVariant::Primary, cx)
            }
            ToggleVariant::Secondary => {
                GranularToggleVariant::from_button_variant(ButtonVariant::Secondary, cx)
            }
            ToggleVariant::Tertiary => {
                GranularToggleVariant::from_button_variant(ButtonVariant::Tertiary, cx)
            }
            ToggleVariant::Constructive => {
                GranularToggleVariant::from_button_variant(ButtonVariant::Constructive, cx)
            }
            ToggleVariant::Destructive => {
                GranularToggleVariant::from_button_variant(ButtonVariant::Destructive, cx)
            }
        }
    }
}

fn falsey_granular_variant(mut variant: GranularButtonVariant) -> GranularButtonVariant {
    variant.bg_color = variant.bg_color.alpha(0.);
    variant.highlight_alpha = 0.;
    variant.bg_hover_color = variant.bg_hover_color.alpha(variant.bg_hover_color.a);
    variant.bg_focus_color = variant.bg_focus_color.alpha(variant.bg_focus_color.a);
    variant
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use gpui::TestAppContext;

    #[gpui::test]
    fn test_toggle_click_behavior_default(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let mut toggle = Toggle::new("test-toggle");
            let behavior = toggle.click_behavior_mut();

            assert!(
                !behavior.allow_propagation,
                "Toggle should not allow propagation by default"
            );
            assert!(
                !behavior.allow_default,
                "Toggle should not allow default by default"
            );
        });
    }

    #[gpui::test]
    fn test_toggle_allow_click_propagation(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let mut toggle = Toggle::new("test-toggle").allow_click_propagation();
            let behavior = toggle.click_behavior_mut();

            assert!(
                behavior.allow_propagation,
                "Toggle should allow propagation after calling allow_click_propagation"
            );
            assert!(
                !behavior.allow_default,
                "Toggle should still not allow default"
            );
        });
    }

    #[gpui::test]
    fn test_toggle_allow_default_click_behaviour(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let mut toggle = Toggle::new("test-toggle").allow_default_click_behaviour();
            let behavior = toggle.click_behavior_mut();

            assert!(
                !behavior.allow_propagation,
                "Toggle should still not allow propagation"
            );
            assert!(
                behavior.allow_default,
                "Toggle should allow default after calling allow_default_click_behaviour"
            );
        });
    }

    #[gpui::test]
    fn test_toggle_click_behavior_chain(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let mut toggle = Toggle::new("test-toggle")
                .allow_click_propagation()
                .allow_default_click_behaviour();
            let behavior = toggle.click_behavior_mut();

            assert!(
                behavior.allow_propagation,
                "Toggle should allow propagation"
            );
            assert!(behavior.allow_default, "Toggle should allow default");
        });
    }
}
