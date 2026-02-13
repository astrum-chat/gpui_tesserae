use gpui::{
    Edges, Hsla, IntoElement, Length, Radians, RenderOnce, SharedString, SizeRefinement, Styled,
    Transformation, prelude::FluentBuilder, px, relative, svg,
};

use crate::theme::ThemeExt;

/// Style configuration for flex layout properties of an Icon.
#[derive(Clone, Default)]
pub struct IconStyle {
    /// The flex grow factor for the icon.
    pub flex_grow: Option<f32>,
    /// The flex shrink factor for the icon.
    pub flex_shrink: Option<f32>,
    /// The flex basis for the icon.
    pub flex_basis: Option<Length>,
}

/// An SVG icon component with configurable size, color, and rotation.
#[derive(IntoElement)]
pub struct Icon {
    path: SharedString,
    pub(crate) size: SizeRefinement<Length>,
    rotate: Radians,
    color: Option<Hsla>,
    style: IconStyle,
    margin: Edges<Option<Length>>,
}

impl Icon {
    /// Creates a new icon from an SVG asset path.
    pub fn new(path: impl Into<SharedString>) -> Self {
        Self {
            path: path.into(),
            size: SizeRefinement::default(),
            rotate: Radians(0.),
            color: None,
            style: IconStyle::default(),
            margin: Edges::default(),
        }
    }

    /// Sets uniform margin for all sides.
    pub fn m(mut self, margin: impl Into<Length>) -> Self {
        let margin = margin.into();
        self.margin = Edges::all(Some(margin));
        self
    }

    /// Sets top margin.
    pub fn mt(mut self, margin: impl Into<Length>) -> Self {
        self.margin.top = Some(margin.into());
        self
    }

    /// Sets bottom margin.
    pub fn mb(mut self, margin: impl Into<Length>) -> Self {
        self.margin.bottom = Some(margin.into());
        self
    }

    /// Sets left margin.
    pub fn ml(mut self, margin: impl Into<Length>) -> Self {
        self.margin.left = Some(margin.into());
        self
    }

    /// Sets right margin.
    pub fn mr(mut self, margin: impl Into<Length>) -> Self {
        self.margin.right = Some(margin.into());
        self
    }

    /// Sets uniform width and height for the icon.
    pub fn size(mut self, size: impl Into<Length>) -> Self {
        let size = size.into();
        self.size = SizeRefinement {
            width: Some(size),
            height: Some(size),
        };
        self
    }

    /// Sets a custom color, overriding the theme's primary text color.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Applies a rotation transformation to the icon.
    pub fn rotate(mut self, rotate: impl Into<Radians>) -> Self {
        self.rotate = rotate.into();
        self
    }

    /// Sets the element to allow a flex item to grow and shrink as needed, ignoring its initial size.
    /// [Docs](https://tailwindcss.com/docs/flex#flex-1)
    pub fn flex_1(mut self) -> Self {
        self.style.flex_grow = Some(1.);
        self.style.flex_shrink = Some(1.);
        self.style.flex_basis = Some(relative(0.).into());
        self
    }

    /// Sets the element to allow a flex item to grow and shrink, taking into account its initial size.
    /// [Docs](https://tailwindcss.com/docs/flex#auto)
    pub fn flex_auto(mut self) -> Self {
        self.style.flex_grow = Some(1.);
        self.style.flex_shrink = Some(1.);
        self.style.flex_basis = Some(Length::Auto);
        self
    }

    /// Sets the element to allow a flex item to shrink but not grow, taking into account its initial size.
    /// [Docs](https://tailwindcss.com/docs/flex#initial)
    pub fn flex_initial(mut self) -> Self {
        self.style.flex_grow = Some(0.);
        self.style.flex_shrink = Some(1.);
        self.style.flex_basis = Some(Length::Auto);
        self
    }

    /// Sets the element to prevent a flex item from growing or shrinking.
    /// [Docs](https://tailwindcss.com/docs/flex#none)
    pub fn flex_none(mut self) -> Self {
        self.style.flex_grow = Some(0.);
        self.style.flex_shrink = Some(0.);
        self
    }

    /// Sets the initial size of flex items for this element.
    /// [Docs](https://tailwindcss.com/docs/flex-basis)
    pub fn flex_basis(mut self, basis: impl Into<Length>) -> Self {
        self.style.flex_basis = Some(basis.into());
        self
    }

    /// Sets the element to allow a flex item to grow to fill any available space.
    /// [Docs](https://tailwindcss.com/docs/flex-grow)
    pub fn flex_grow(mut self) -> Self {
        self.style.flex_grow = Some(1.);
        self
    }

    /// Sets the flex grow factor to a specific value.
    pub fn flex_grow_factor(mut self, value: f32) -> Self {
        self.style.flex_grow = Some(value);
        self
    }

    /// Sets the element to allow a flex item to shrink if needed.
    /// [Docs](https://tailwindcss.com/docs/flex-shrink)
    pub fn flex_shrink(mut self) -> Self {
        self.style.flex_shrink = Some(1.);
        self
    }

    /// Sets the flex shrink factor to a specific value.
    pub fn flex_shrink_factor(mut self, value: f32) -> Self {
        self.style.flex_shrink = Some(value);
        self
    }

    /// Sets the element to prevent a flex item from shrinking.
    /// [Docs](https://tailwindcss.com/docs/flex-shrink#dont-shrink)
    pub fn flex_shrink_0(mut self) -> Self {
        self.style.flex_shrink = Some(0.);
        self
    }

    /// Applies an IconStyle to this icon.
    pub fn style(mut self, style: IconStyle) -> Self {
        self.style = style;
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let primary_text_color = cx.get_theme().variants.active(cx).colors.text.primary;
        let size = self.size;
        let width = size.width.unwrap_or(px(14.).into());
        let height = size.height.unwrap_or(px(14.).into());

        svg()
            .path(self.path)
            .text_color(primary_text_color)
            .w(width)
            .min_w(width)
            .h(height)
            .min_h(height)
            .when_some(self.margin.top, |this, v| this.mt(v))
            .when_some(self.margin.bottom, |this, v| this.mb(v))
            .when_some(self.margin.left, |this, v| this.ml(v))
            .when_some(self.margin.right, |this, v| this.mr(v))
            .with_transformation(Transformation::rotate(self.rotate))
            .when_some(self.color, |this, color| this.text_color(color))
            .when_some(self.style.flex_grow, |mut this, value| {
                this.style().flex_grow = Some(value);
                this
            })
            .when_some(self.style.flex_shrink, |mut this, value| {
                this.style().flex_shrink = Some(value);
                this
            })
            .when_some(self.style.flex_basis, |this, value| this.flex_basis(value))
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use gpui::{AppContext, ParentElement, TestAppContext, VisualTestContext, hsla};

    #[gpui::test]
    fn test_icon_creation(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let icon = Icon::new("icons/test.svg");
            assert_eq!(icon.path, SharedString::from("icons/test.svg"));
            assert!(icon.color.is_none(), "Icon should start with no color");
            assert_eq!(icon.rotate.0, 0.0, "Icon should start with no rotation");
        });
    }

    #[gpui::test]
    fn test_icon_size(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let icon = Icon::new("icons/test.svg").size(px(24.));
            assert!(icon.size.width.is_some(), "Icon should have width");
            assert!(icon.size.height.is_some(), "Icon should have height");
        });
    }

    #[gpui::test]
    fn test_icon_color(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let color = hsla(0.5, 0.5, 0.5, 1.0);
            let icon = Icon::new("icons/test.svg").color(color);
            assert!(icon.color.is_some(), "Icon should have color");
        });
    }

    #[gpui::test]
    fn test_icon_rotation(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let icon = Icon::new("icons/test.svg").rotate(Radians(std::f32::consts::PI));
            assert_eq!(
                icon.rotate.0,
                std::f32::consts::PI,
                "Icon should have rotation"
            );
        });
    }

    #[gpui::test]
    fn test_icon_builder_chain(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let color = hsla(0.5, 0.5, 0.5, 1.0);
            let icon = Icon::new("icons/test.svg")
                .size(px(32.))
                .color(color)
                .rotate(Radians(1.5));

            assert!(icon.size.width.is_some());
            assert!(icon.color.is_some());
            assert_eq!(icon.rotate.0, 1.5);
        });
    }

    #[gpui::test]
    fn test_icon_renders_in_window(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};

        let window = cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            cx.open_window(Default::default(), |_window, cx| cx.new(|_cx| IconTestView))
                .unwrap()
        });

        let _cx = VisualTestContext::from_window(window.into(), cx);

        // The window creation itself validates rendering works
    }

    /// Test view that contains an Icon
    struct IconTestView;

    impl gpui::Render for IconTestView {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            gpui::div()
                .size_full()
                .child(Icon::new("icons/test.svg").size(px(24.)))
        }
    }
}
