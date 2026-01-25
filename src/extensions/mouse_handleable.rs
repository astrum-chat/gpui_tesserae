use gpui::{App, ClickEvent, MouseButton, MouseDownEvent, MouseUpEvent, Window};

/// Type alias for the on_click callback that receives a ClickEvent.
pub type OnClickHandler<CE> = Box<dyn Fn(&CE, &mut Window, &mut App) + 'static>;

/// Type alias for the on_mouse_down callback that receives a MouseDownEvent.
pub type OnMouseDownHandler = Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

/// Type alias for the on_mouse_up callback that receives a MouseUpEvent.
pub type OnMouseUpHandler = Box<dyn Fn(&MouseUpEvent, &mut Window, &mut App) + 'static>;

/// A struct that holds mouse-related event handlers.
///
/// This struct encapsulates the various mouse event handlers that can be
/// attached to interactive components.
#[derive(Default)]
pub struct MouseHandlers<CE: Default = ClickEvent> {
    /// Handler called when the element is clicked (mouse down + up within bounds).
    pub on_click: Option<OnClickHandler<CE>>,
    /// Handler called when a specific mouse button is pressed down on the element.
    pub on_mouse_down: Option<(MouseButton, OnMouseDownHandler)>,
    /// Handler called when a specific mouse button is released on the element.
    pub on_mouse_up: Option<(MouseButton, OnMouseUpHandler)>,
    /// Handler called when any mouse button is pressed down on the element.
    pub on_any_mouse_down: Option<OnMouseDownHandler>,
    /// Handler called when any mouse button is released on the element.
    pub on_any_mouse_up: Option<OnMouseUpHandler>,
}

impl<CE: Default> MouseHandlers<CE> {
    /// Creates a new empty MouseHandlers instance.
    pub fn new() -> Self {
        Self::default()
    }
}

/// A trait for components that support mouse-related event handlers.
///
/// Implement this trait to add support for `on_click`, `on_mouse_down`, `on_mouse_up`,
/// `on_any_mouse_down`, and `on_any_mouse_up` handlers to your component.
pub trait MouseHandleable<CE: Default = ClickEvent>: Sized {
    /// Returns a mutable reference to the mouse handlers.
    fn mouse_handlers_mut(&mut self) -> &mut MouseHandlers<CE>;

    /// Sets the on_click handler.
    ///
    /// The handler is called when the element is clicked (mouse down + up within bounds).
    fn on_click(mut self, handler: impl Fn(&CE, &mut Window, &mut App) + 'static) -> Self {
        self.mouse_handlers_mut().on_click = Some(Box::new(handler));
        self
    }

    /// Sets the on_mouse_down handler for a specific mouse button.
    ///
    /// The handler is called when the specified mouse button is pressed down on the element.
    fn on_mouse_down(
        mut self,
        button: MouseButton,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.mouse_handlers_mut().on_mouse_down = Some((button, Box::new(handler)));
        self
    }

    /// Sets the on_mouse_up handler for a specific mouse button.
    ///
    /// The handler is called when the specified mouse button is released on the element.
    fn on_mouse_up(
        mut self,
        button: MouseButton,
        handler: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.mouse_handlers_mut().on_mouse_up = Some((button, Box::new(handler)));
        self
    }

    /// Sets the on_any_mouse_down handler.
    ///
    /// The handler is called when any mouse button is pressed down on the element.
    fn on_any_mouse_down(
        mut self,
        handler: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.mouse_handlers_mut().on_any_mouse_down = Some(Box::new(handler));
        self
    }

    /// Sets the on_any_mouse_up handler.
    ///
    /// The handler is called when any mouse button is released on the element.
    fn on_any_mouse_up(
        mut self,
        handler: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.mouse_handlers_mut().on_any_mouse_up = Some(Box::new(handler));
        self
    }
}
