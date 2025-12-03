use gpui::{AnyElement, IntoElement};

pub trait PositionalParentElement: Sized {
    fn children_mut(&mut self) -> &mut PositionalChildren;

    fn child_top(mut self, child: impl IntoElement) -> Self {
        self.children_mut().top.push(child.into_any_element());
        self
    }

    fn child_bottom(mut self, child: impl IntoElement) -> Self {
        self.children_mut().bottom.push(child.into_any_element());
        self
    }

    fn child_left(mut self, child: impl IntoElement) -> Self {
        self.children_mut().left.push(child.into_any_element());
        self
    }

    fn child_right(mut self, child: impl IntoElement) -> Self {
        self.children_mut().right.push(child.into_any_element());
        self
    }
}

#[derive(Default)]
pub struct PositionalChildren {
    pub top: Vec<AnyElement>,
    pub bottom: Vec<AnyElement>,
    pub left: Vec<AnyElement>,
    pub right: Vec<AnyElement>,
}
