use gpui::{AnyElement, IntoElement};

/// Trait for elements that support adding children at specific positions.
pub trait PositionalParentElement: Sized {
    /// Returns a mutable reference to the positional children container.
    fn children_mut(&mut self) -> &mut PositionalChildren;

    /// Adds a child element to the top position.
    fn child_top(mut self, child: impl IntoElement) -> Self {
        self.children_mut().top.push(child.into_any_element());
        self
    }

    /// Adds a child element to the bottom position.
    fn child_bottom(mut self, child: impl IntoElement) -> Self {
        self.children_mut().bottom.push(child.into_any_element());
        self
    }

    /// Adds a child element to the left position.
    fn child_left(mut self, child: impl IntoElement) -> Self {
        self.children_mut().left.push(child.into_any_element());
        self
    }

    /// Adds a child element to the right position.
    fn child_right(mut self, child: impl IntoElement) -> Self {
        self.children_mut().right.push(child.into_any_element());
        self
    }
}

/// Container for child elements organized by position.
#[derive(Default)]
pub struct PositionalChildren {
    /// Children positioned at the top.
    pub top: Vec<AnyElement>,
    /// Children positioned at the bottom.
    pub bottom: Vec<AnyElement>,
    /// Children positioned at the left.
    pub left: Vec<AnyElement>,
    /// Children positioned at the right.
    pub right: Vec<AnyElement>,
}
