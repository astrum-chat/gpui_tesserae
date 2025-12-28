use gpui::{
    App, DefiniteLength, ElementId, IntoElement, Length, Pixels, Radians, RenderOnce, SharedString,
    Window, prelude::FluentBuilder,
};

use crate::{
    PositionalParentElement,
    components::{Button, ButtonVariant, GranularButtonVariant},
    utils::RgbaExt,
};

#[derive(IntoElement)]
pub struct Toggle {
    variant: ToggleVariantEither,
    checked: bool,
    on_click: Option<Box<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    base: Button,
}

impl Toggle {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            checked: false,
            variant: ToggleVariantEither::Left(ToggleVariant::Primary),
            on_click: None,
            base: Button::new(id),
        }
    }

    pub fn text(mut self, text: impl Into<SharedString>) -> Self {
        self.base = self.base.text(text);
        self
    }

    pub fn no_text(mut self) -> Self {
        self.base = self.base.no_text();
        self
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.base = self.base.icon(icon);
        self
    }

    pub fn icon_size(mut self, icon_size: impl Into<Length>) -> Self {
        self.base = self.base.icon_size(icon_size);
        self
    }

    pub fn icon_rotation(mut self, rotate: impl Into<Radians>) -> Self {
        self.base = self.base.icon_rotate(rotate);
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.base = self.base.disabled(disabled);
        self
    }

    pub fn on_hover(mut self, on_hover: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.base = self.base.on_hover(on_hover);
        self
    }

    pub fn on_click(mut self, on_click: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(on_click));
        self
    }

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

    pub fn rounded(mut self, rounded: impl Into<Pixels>) -> Self {
        self.base = self.base.rounded(rounded);
        self
    }

    pub fn rounded_tl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.base = self.base.rounded_tl(rounded);
        self
    }

    pub fn rounded_tr(mut self, rounded: impl Into<Pixels>) -> Self {
        self.base = self.base.rounded_tr(rounded);
        self
    }

    pub fn rounded_bl(mut self, rounded: impl Into<Pixels>) -> Self {
        self.base = self.base.rounded_bl(rounded);
        self
    }

    pub fn rounded_br(mut self, rounded: impl Into<Pixels>) -> Self {
        self.base = self.base.rounded_br(rounded);
        self
    }

    pub fn p(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.p(padding);
        self
    }

    pub fn pt(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.pt(padding);
        self
    }

    pub fn pb(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.pb(padding);
        self
    }

    pub fn pl(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.pl(padding);
        self
    }

    pub fn pr(mut self, padding: impl Into<DefiniteLength>) -> Self {
        self.base = self.base.pr(padding);
        self
    }

    pub fn w(mut self, width: impl Into<Length>) -> Self {
        self.base = self.base.w(width);
        self
    }

    pub fn w_auto(mut self) -> Self {
        self.base = self.base.w_auto();
        self
    }

    pub fn w_full(mut self) -> Self {
        self.base = self.base.w_full();
        self
    }
}

impl RenderOnce for Toggle {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let is_checked = self.checked;

        let variant = self.variant.into_granular(cx);

        self.base
            .variant(if is_checked {
                variant.truthy
            } else {
                variant.falsey
            })
            .when_some(self.on_click, |this, on_click| {
                this.on_click(move |_event, cx, window| (on_click)(&!self.checked, cx, window))
            })
    }
}

impl PositionalParentElement for Toggle {
    fn children_mut(&mut self) -> &mut crate::utils::PositionalChildren {
        self.base.children_mut()
    }
}

#[derive(Clone, Copy)]
pub enum ToggleVariant {
    Primary,
    Secondary,
    Tertiary,
    Constructive,
    Destructive,
}

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
