use gpui::{
    Hsla, IntoElement, Length, Radians, RenderOnce, SharedString, SizeRefinement, Styled,
    Transformation, prelude::FluentBuilder, px, svg,
};

use crate::theme::ThemeExt;

#[derive(IntoElement)]
pub struct Icon {
    path: SharedString,
    pub(crate) size: SizeRefinement<Length>,
    rotate: Radians,
    color: Option<Hsla>,
}

impl Icon {
    pub fn new(path: impl Into<SharedString>) -> Self {
        Self {
            path: path.into(),
            size: SizeRefinement::default(),
            rotate: Radians(0.),
            color: None,
        }
    }

    pub fn size(mut self, size: impl Into<Length>) -> Self {
        let size = size.into();
        self.size = SizeRefinement {
            width: Some(size),
            height: Some(size),
        };
        self
    }

    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn rotate(mut self, rotate: impl Into<Radians>) -> Self {
        self.rotate = rotate.into();
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
            .with_transformation(Transformation::rotate(self.rotate))
            .when_some(self.color, |this, color| this.text_color(color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{TestAppContext, VisualTestContext, hsla};

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
