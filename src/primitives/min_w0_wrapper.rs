use gpui::{AnyElement, StyleRefinement, div, prelude::*};
use smallvec::SmallVec;

#[derive(IntoElement)]
pub struct MinW0Wrapper {
    children: SmallVec<[AnyElement; 2]>,
    style: StyleRefinement,
}

impl MinW0Wrapper {
    pub fn new() -> Self {
        Self {
            children: SmallVec::new(),
            style: StyleRefinement::default().w_auto().h_auto().min_w_0(),
        }
    }
}

impl RenderOnce for MinW0Wrapper {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        div()
            .map(|mut this| {
                this.style().refine(&self.style);
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

pub fn min_w0_wrapper() -> MinW0Wrapper {
    MinW0Wrapper::new()
}
