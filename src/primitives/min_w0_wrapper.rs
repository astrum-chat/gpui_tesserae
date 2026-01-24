use gpui::{AnyElement, StyleRefinement, div, prelude::*};
use smallvec::SmallVec;

use crate::theme::ThemeExt;

/// A wrapper element that sets `min-width: 0` to allow flex children to shrink.
///
/// In flexbox layouts, children have an implicit minimum size based on their content.
/// This wrapper overrides that behavior, allowing text to truncate properly instead
/// of forcing the container to expand.
#[derive(IntoElement)]
pub struct MinW0Wrapper {
    children: SmallVec<[AnyElement; 2]>,
    style: StyleRefinement,
}

impl MinW0Wrapper {
    /// Creates a new wrapper with default theme styling applied.
    pub fn new() -> Self {
        Self {
            children: SmallVec::new(),
            style: StyleRefinement::default().w_auto().h_auto().min_w_0(),
        }
    }
}

impl RenderOnce for MinW0Wrapper {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let mut style = self.style;

        style
            .text
            .font_family
            .get_or_insert_with(|| cx.get_theme().layout.text.default_font.family[0].clone());

        style
            .text
            .font_size
            .get_or_insert_with(|| cx.get_theme().layout.text.default_font.sizes.body);

        style.text.color.get_or_insert_with(|| {
            cx.get_theme()
                .variants
                .active(cx)
                .colors
                .text
                .secondary
                .into()
        });

        div()
            .map(|mut this| {
                this.style().refine(&style);
                this
            })
            .children(self.children)
    }
}

impl ParentElement for MinW0Wrapper {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for MinW0Wrapper {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// Creates a new [`MinW0Wrapper`] element.
pub fn min_w0_wrapper() -> MinW0Wrapper {
    MinW0Wrapper::new()
}
