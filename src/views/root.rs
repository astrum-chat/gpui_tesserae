use std::{any::TypeId, cell::RefCell, cmp::Ordering, collections::BTreeMap, rc::Rc};

use gpui::{
    AnyElement, AnyView, App, Bounds, ClickEvent, Context, ElementId, InteractiveElement,
    IntoElement, Length, MouseDownEvent, MouseUpEvent, ParentElement, Render,
    StatefulInteractiveElement, Styled, Window, WindowHandle, div, prelude::FluentBuilder, px,
};

#[derive(PartialEq, Eq, Hash)]
struct ElementIdKey(ElementId);

fn variant_index(id: &ElementId) -> u8 {
    use ElementId::*;
    match id {
        View(_) => 0,
        Integer(_) => 1,
        Name(_) => 2,
        Uuid(_) => 3,
        FocusHandle(_) => 4,
        NamedInteger(_, _) => 5,
        Path(_) => 6,
        CodeLocation(_) => 7,
        NamedChild(_, _) => 8,
    }
}

impl PartialOrd for ElementIdKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ElementIdKey {
    fn cmp(&self, other: &Self) -> Ordering {
        use ElementId::*;
        match (&self.0, &other.0) {
            (View(a), View(b)) => format!("{:?}", a).cmp(&format!("{:?}", b)),
            (Integer(a), Integer(b)) => a.cmp(b),
            (Name(a), Name(b)) => a.as_ref().cmp(b.as_ref()),
            (Uuid(a), Uuid(b)) => a.cmp(b),
            (FocusHandle(a), FocusHandle(b)) => format!("{:?}", a).cmp(&format!("{:?}", b)),
            (NamedInteger(a1, a2), NamedInteger(b1, b2)) => {
                a1.as_ref().cmp(b1.as_ref()).then_with(|| a2.cmp(b2))
            }
            (Path(a), Path(b)) => a.cmp(b),
            (CodeLocation(a), CodeLocation(b)) => a
                .file()
                .cmp(b.file())
                .then_with(|| a.line().cmp(&b.line()))
                .then_with(|| a.column().cmp(&b.column())),
            (NamedChild(a1, a2), NamedChild(b1, b2)) => ElementIdKey(a1.as_ref().clone())
                .cmp(&ElementIdKey(b1.as_ref().clone()))
                .then_with(|| a2.as_ref().cmp(b2.as_ref())),
            _ => variant_index(&self.0).cmp(&variant_index(&other.0)),
        }
    }
}

/// Represents an overlay with bounds and the element to render.
pub struct OverlayEntry {
    /// Unique identifier for this overlay.
    pub id: ElementId,
    /// Position and size of the overlay within the root.
    pub bounds: Bounds<Length>,
    /// Factory function that creates the overlay element.
    pub element: Box<dyn FnOnce(&mut Window, &mut App) -> AnyElement + 'static>,
}

impl OverlayEntry {
    /// Creates a new overlay with the given bounds and element.
    pub fn new(
        id: impl Into<ElementId>,
        bounds: Bounds<Length>,
        element: impl FnOnce(&mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            bounds,
            element: Box::new(element),
        }
    }
}

/// Root is the top-level view component that renders a child view and any overlay elements.
///
/// Overlay elements are rendered in front of everything else, positioned absolutely
/// within the Root's bounds.
///
/// # Example
///
/// ```ignore
/// // In your window creation:
/// cx.open_window(options, |window, cx| {
///     cx.new(|cx| Root::new(your_main_view, window, cx))
/// });
///
/// // To add an overlay from anywhere:
/// let root = window
///     .root::<Root>()
///     .expect("The window's root view should be of type `gpui_tesserae::Root`")
///     .unwrap();
///
/// root.update(cx, |root, cx| {
///     root.add(
///         Bounds::new(point(px(100.), px(100.)), size(px(200.), px(150.))),
///         |_window, _cx| {
///             div().size_full().bg(red())
///         },
///     );
///     cx.notify();
/// });
/// ```
pub struct Root {
    view: AnyView,
    overlays: BTreeMap<ElementIdKey, OverlayEntry>,
    mouse_events: MouseEvents,
}

struct MouseEvents {
    on_click: Rc<RefCell<Vec<Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>>>,
    on_mouse_down: Rc<RefCell<Vec<Box<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>>>>,
    on_mouse_up: Rc<RefCell<Vec<Box<dyn Fn(&MouseUpEvent, &mut Window, &mut App) + 'static>>>>,
}

impl MouseEvents {
    fn new() -> Self {
        Self {
            on_click: Rc::new(RefCell::new(vec![])),
            on_mouse_down: Rc::new(RefCell::new(vec![])),
            on_mouse_up: Rc::new(RefCell::new(vec![])),
        }
    }

    fn is_empty(&self) -> bool {
        self.on_click.borrow().is_empty()
            && self.on_mouse_down.borrow().is_empty()
            && self.on_mouse_up.borrow().is_empty()
    }
}

impl Root {
    /// Creates a new Root with the given child view.
    pub fn new(view: impl Into<AnyView>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            view: view.into(),
            overlays: BTreeMap::new(),
            mouse_events: MouseEvents::new(),
        }
    }

    /// Adds an overlay with the specified bounds. Returns the overlay ID.
    pub fn add<E: IntoElement>(
        &mut self,
        id: impl Into<ElementId>,
        bounds: Bounds<Length>,
        element: impl FnOnce(&mut Window, &mut App) -> E + 'static,
    ) {
        let id = id.into();

        let overlay = OverlayEntry::new(id.clone(), bounds, |window, cx| {
            element(window, cx).into_any_element()
        });
        self.overlays.insert(ElementIdKey(id), overlay);
    }

    /// Removes an overlay by its ID. Returns true if found and removed.
    pub fn remove(&mut self, id: impl Into<ElementId>) -> bool {
        self.overlays.remove(&ElementIdKey(id.into())).is_some()
    }

    /// Clears all overlay entries.
    pub fn clear(&mut self) {
        self.overlays.clear();
    }

    /// Registers a click handler that fires for any click on the root overlay.
    pub fn on_click(
        &mut self,
        on_mouse_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_events
            .on_click
            .borrow_mut()
            .push(Box::new(on_mouse_click));
    }

    /// Registers a mouse down handler that fires for any mouse down on the root overlay.
    pub fn on_any_mouse_down(
        &mut self,
        on_mouse_down: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_events
            .on_mouse_down
            .borrow_mut()
            .push(Box::new(on_mouse_down));
    }

    /// Registers a mouse up handler that fires for any mouse up on the root overlay.
    pub fn on_any_mouse_up(
        &mut self,
        on_mouse_up: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
    ) {
        self.mouse_events
            .on_mouse_up
            .borrow_mut()
            .push(Box::new(on_mouse_up));
    }

    /// Returns whether any mouse event handlers are registered.
    pub fn click_overlay_visible(&self) -> bool {
        !self.mouse_events.is_empty()
    }

    /// Finds a Root window with a specific child view.
    pub fn find_window<V: Render>(cx: &App) -> Option<WindowHandle<Root>> {
        cx.windows().iter().find_map(|window| {
            let window = window.downcast::<Root>();

            let is_of_view = window
                .map(|root| root.read(cx).ok().map(|this| this.is_of_view::<V>()))
                .flatten()
                .unwrap_or(false);

            if is_of_view { window } else { None }
        })
    }

    /// Checks if the child view of this root is of the specific type.
    pub fn is_of_view<'a, V: Render>(&self) -> bool {
        TypeId::of::<V>() == self.view.entity_type()
    }

    /// Takes all entries, leaving the internal list empty.
    fn take_overlays(&mut self) -> BTreeMap<ElementIdKey, OverlayEntry> {
        std::mem::take(&mut self.overlays)
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let overlays_is_empty = self.overlays.is_empty();
        let mouse_events_is_empty = self.mouse_events.is_empty();

        div()
            .id("root")
            .size_full()
            .relative()
            .child(self.view.clone())
            .when(!(overlays_is_empty && mouse_events_is_empty), |this| {
                let overlays = self.take_overlays();

                this.child(
                    div()
                        .id("root-overlay-container")
                        .absolute()
                        .top(px(0.))
                        .left(px(0.))
                        .size_full()
                        .children(overlays.into_values().enumerate().map(|(idx, overlay)| {
                            let element = (overlay.element)(window, cx);

                            div()
                                .id(format!("overlay-item-{}", idx))
                                .absolute()
                                .top(overlay.bounds.origin.y)
                                .left(overlay.bounds.origin.x)
                                .w(overlay.bounds.size.width)
                                .h(overlay.bounds.size.height)
                                .child(element)
                        })),
                )
                .when(!mouse_events_is_empty, |this| {
                    this.child(
                        div()
                            .id(format!("root-click-overlay"))
                            //.bg(gpui::red())
                            .absolute()
                            .size_full()
                            .map(|this| {
                                let on_click = std::mem::take(&mut self.mouse_events.on_click);

                                if on_click.borrow().len() != 0 {
                                    this.on_click(move |event, window, cx| {
                                        for callback in on_click.borrow().iter() {
                                            (callback)(event, window, cx)
                                        }
                                    })
                                } else {
                                    this
                                }
                            })
                            .map(|this| {
                                let on_mouse_down =
                                    std::mem::take(&mut self.mouse_events.on_mouse_down);

                                if on_mouse_down.borrow().len() != 0 {
                                    this.on_any_mouse_down(move |event, window, cx| {
                                        for callback in on_mouse_down.borrow().iter() {
                                            (callback)(event, window, cx)
                                        }
                                    })
                                } else {
                                    this
                                }
                            })
                            .map(|mut this| {
                                let on_mouse_up =
                                    std::mem::take(&mut self.mouse_events.on_mouse_up);

                                if on_mouse_up.borrow().len() != 0 {
                                    let interactivity = this.interactivity();

                                    interactivity.on_any_mouse_up(move |event, window, cx| {
                                        for callback in on_mouse_up.borrow().iter() {
                                            (callback)(event, window, cx)
                                        }
                                    });

                                    this
                                } else {
                                    this
                                }
                            }),
                    )
                })
            })
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use gpui::{AppContext, TestAppContext, VisualTestContext, point, size};

    /// A simple test view for use in Root tests.
    struct TestView;

    impl Render for TestView {
        fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
            div().id("test-view").size_full().child("Test Content")
        }
    }

    #[gpui::test]
    fn test_root_creation(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let test_view = cx.new(|_cx| TestView);
                cx.new(|cx| Root::new(test_view, window, cx))
            })
            .unwrap()
        });

        let root = window.root(cx).unwrap();
        root.read_with(cx, |root, _| {
            assert!(
                root.overlays.is_empty(),
                "Root should start with no overlays"
            );
        });
    }

    #[gpui::test]
    fn test_root_add_overlay(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let test_view = cx.new(|_cx| TestView);
                cx.new(|cx| Root::new(test_view, window, cx))
            })
            .unwrap()
        });

        let root = window.root(cx).unwrap();

        // Add an overlay
        root.update(cx, |root, _cx| {
            root.add(
                "overlay",
                Bounds::new(
                    point(px(100.).into(), px(100.).into()),
                    size(px(200.).into(), px(150.).into()),
                ),
                |_window, _cx| div().child("Overlay"),
            )
        });

        root.read_with(cx, |root, _| {
            assert_eq!(root.overlays.len(), 1, "Should have one overlay");
            assert_eq!(
                root.overlays.values().nth(0).unwrap().id,
                "overlay".into(),
                "Overlay ID should match"
            );
        });
    }

    #[gpui::test]
    fn test_root_add_multiple_overlays(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let test_view = cx.new(|_cx| TestView);
                cx.new(|cx| Root::new(test_view, window, cx))
            })
            .unwrap()
        });

        let root = window.root(cx).unwrap();

        // Add multiple overlays
        root.update(cx, |root, _cx| {
            root.add(
                "overlay_1",
                Bounds::new(
                    point(px(0.).into(), px(0.).into()),
                    size(px(100.).into(), px(100.).into()),
                ),
                |_window, _cx| div().child("Overlay 1"),
            )
        });

        root.update(cx, |root, _cx| {
            root.add(
                "overlay_2",
                Bounds::new(
                    point(px(50.).into(), px(50.).into()),
                    size(px(100.).into(), px(100.).into()),
                ),
                |_window, _cx| div().child("Overlay 2"),
            )
        });

        root.read_with(cx, |root, _| {
            assert_eq!(root.overlays.len(), 2, "Should have two overlays");
            assert_ne!(
                root.overlays.values().nth(0).unwrap().id,
                root.overlays.values().nth(1).unwrap().id,
                "Overlay IDs should be unique"
            );
        });
    }

    #[gpui::test]
    fn test_root_remove_overlay(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let test_view = cx.new(|_cx| TestView);
                cx.new(|cx| Root::new(test_view, window, cx))
            })
            .unwrap()
        });

        let root = window.root(cx).unwrap();

        // Add an overlay
        root.update(cx, |root, _cx| {
            root.add(
                "overlay",
                Bounds::new(
                    point(px(100.).into(), px(100.).into()),
                    size(px(200.).into(), px(150.).into()),
                ),
                |_window, _cx| div().child("Overlay"),
            )
        });

        // Remove it
        let removed = root.update(cx, |root, _cx| root.remove("overlay"));
        assert!(removed, "Remove should return true for existing overlay");

        root.read_with(cx, |root, _| {
            assert!(
                root.overlays.is_empty(),
                "Should have no overlays after removal"
            );
        });

        // Try to remove again (should fail)
        let removed_again = root.update(cx, |root, _cx| root.remove("overlay"));
        assert!(
            !removed_again,
            "Remove should return false for non-existent overlay"
        );
    }

    #[gpui::test]
    fn test_root_clear_overlays(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let test_view = cx.new(|_cx| TestView);
                cx.new(|cx| Root::new(test_view, window, cx))
            })
            .unwrap()
        });

        let root = window.root(cx).unwrap();

        // Add several overlays
        root.update(cx, |root, _cx| {
            for idx in 0..5 {
                root.add(
                    format!("overlay_{}", idx),
                    Bounds::new(
                        point(px(idx as f32 * 10.).into(), px(idx as f32 * 10.).into()),
                        size(px(50.).into(), px(50.).into()),
                    ),
                    move |_window, _cx| div().child(format!("Overlay {}", idx)),
                );
            }
        });

        root.read_with(cx, |root, _| {
            assert_eq!(root.overlays.len(), 5, "Should have five overlays");
        });

        // Clear all
        root.update(cx, |root, _cx| root.clear());

        root.read_with(cx, |root, _| {
            assert!(
                root.overlays.is_empty(),
                "Should have no overlays after clear"
            );
        });
    }

    #[gpui::test]
    fn test_root_overlay_bounds(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let test_view = cx.new(|_cx| TestView);
                cx.new(|cx| Root::new(test_view, window, cx))
            })
            .unwrap()
        });

        let root = window.root(cx).unwrap();

        let expected_bounds = Bounds::new(
            point(px(42.).into(), px(84.).into()),
            size(px(200.).into(), px(300.).into()),
        );

        root.update(cx, |root, _cx| {
            root.add("overlay", expected_bounds, |_window, _cx| {
                div().child("Overlay")
            });
        });

        root.read_with(cx, |root, _| {
            let overlay = &root.overlays.values().nth(0).unwrap();
            assert_eq!(
                overlay.bounds.origin.x,
                Length::Definite(px(42.).into()),
                "X origin should match"
            );
            assert_eq!(
                overlay.bounds.origin.y,
                Length::Definite(px(84.).into()),
                "Y origin should match"
            );
            assert_eq!(
                overlay.bounds.size.width,
                Length::Definite(px(200.).into()),
                "Width should match"
            );
            assert_eq!(
                overlay.bounds.size.height,
                Length::Definite(px(300.).into()),
                "Height should match"
            );
        });
    }

    #[gpui::test]
    fn test_root_renders_in_window(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let test_view = cx.new(|_cx| TestView);
                cx.new(|cx| Root::new(test_view, window, cx))
            })
            .unwrap()
        });

        let _cx = VisualTestContext::from_window(window.into(), cx);
    }

    #[gpui::test]
    fn test_root_renders_with_overlays(cx: &mut TestAppContext) {
        let window = cx.update(|cx| {
            cx.open_window(Default::default(), |window, cx| {
                let test_view = cx.new(|_cx| TestView);
                cx.new(|cx| Root::new(test_view, window, cx))
            })
            .unwrap()
        });

        let root = window.root(cx).unwrap();

        // Add an overlay before rendering
        root.update(cx, |root, cx| {
            root.add(
                "overlay",
                Bounds::new(
                    point(px(10.).into(), px(10.).into()),
                    size(px(100.).into(), px(100.).into()),
                ),
                |_window, _cx| div().id("overlay-content").child("Overlay"),
            );
            cx.notify();
        });

        let _cx = VisualTestContext::from_window(window.into(), cx);

        // The window creation itself validates rendering works
    }
}
