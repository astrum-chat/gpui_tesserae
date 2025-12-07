use gpui::{
    Hsla, IntoElement, Length, RenderOnce, SharedString, SizeRefinement, Styled,
    prelude::FluentBuilder, px, svg,
};
use gpui_tesserae_theme::ThemeExt;

#[derive(IntoElement)]
pub struct Icon {
    path: SharedString,
    pub(crate) size: SizeRefinement<Length>,
    color: Option<Hsla>,
}

impl Icon {
    pub fn new(path: impl Into<SharedString>) -> Self {
        Self {
            path: path.into(),
            size: SizeRefinement::default(),
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
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let primary_text_color = cx.get_theme().variants.active().colors.text.primary;
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
            .when_some(self.color, |this, color| this.text_color(color))
    }
}
