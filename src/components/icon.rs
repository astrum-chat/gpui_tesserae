use gpui::{IntoElement, RenderOnce, SharedString, Styled, px, svg};
use gpui_tesserae_theme::ThemeExt;

#[derive(IntoElement)]
pub struct Icon {
    path: SharedString,
}

impl Icon {
    pub fn new(path: impl Into<SharedString>) -> Self {
        Self { path: path.into() }
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let primary_text_color = cx.get_theme().variants.active().colors.text.primary;

        svg()
            .path(self.path)
            .text_color(primary_text_color)
            .size(px(14.))
            .min_w(px(14.))
            .min_h(px(14.))
    }
}
