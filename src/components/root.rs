use std::sync::atomic::{AtomicU64, Ordering};

use gpui::{
    AnyElement, AnyView, App, Bounds, Context, ElementId, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, Styled, Window, div, prelude::FluentBuilder, px,
};

static NEXT_OVERLAY_ID: AtomicU64 = AtomicU64::new(0);

/// Generates a unique ID for overlay elements.
fn next_overlay_id() -> u64 {
    NEXT_OVERLAY_ID.fetch_add(1, Ordering::SeqCst)
}

/// Represents a single overlay entry with bounds and the element to render.
pub struct OverlayEntry {
    pub id: u64,
    pub bounds: Bounds<Pixels>,
    pub element: Box<dyn FnOnce(&mut Window, &mut App) -> AnyElement + Send + 'static>,
}

impl OverlayEntry {
    /// Creates a new overlay entry with the given bounds and element.
    pub fn new(
        bounds: Bounds<Pixels>,
        element: impl FnOnce(&mut Window, &mut App) -> AnyElement + Send + 'static,
    ) -> Self {
        Self {
            id: next_overlay_id(),
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
///             div().size_full().bg(red()).into_any_element()
///         },
///     );
///     cx.notify();
/// });
/// ```
pub struct Root {
    view: AnyView,
    entries: Vec<OverlayEntry>,
}

impl Root {
    /// Creates a new Root with the given child view.
    pub fn new(view: impl Into<AnyView>, _window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {
            view: view.into(),
            entries: Vec::new(),
        }
    }

    /// Adds an overlay with the specified bounds. Returns the overlay ID.
    pub fn add(
        &mut self,
        bounds: Bounds<Pixels>,
        element: impl FnOnce(&mut Window, &mut App) -> AnyElement + Send + 'static,
    ) -> u64 {
        let entry = OverlayEntry::new(bounds, element);
        let id = entry.id;
        self.entries.push(entry);
        id
    }

    /// Removes an overlay by its ID. Returns true if found and removed.
    pub fn remove(&mut self, id: u64) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    /// Clears all overlay entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Takes all entries, leaving the internal list empty.
    fn take_entries(&mut self) -> Vec<OverlayEntry> {
        std::mem::take(&mut self.entries)
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Take all overlay entries
        let overlay_entries = self.take_entries();

        // Collect rendered overlay elements
        let mut overlay_elements: Vec<AnyElement> = Vec::new();
        for (index, entry) in overlay_entries.into_iter().enumerate() {
            let element = (entry.element)(window, cx);

            let overlay_div = div()
                .id(ElementId::Name(format!("overlay-item-{}", index).into()))
                .absolute()
                .top(entry.bounds.origin.y)
                .left(entry.bounds.origin.x)
                .w(entry.bounds.size.width)
                .h(entry.bounds.size.height)
                .child(element);

            overlay_elements.push(overlay_div.into_any_element());
        }

        div()
            .id("root")
            .size_full()
            .relative()
            // Render main child view
            .child(self.view.clone())
            // Render overlay container on top
            .when(!overlay_elements.is_empty(), |this| {
                this.child(
                    div()
                        .id("root-overlay-container")
                        .absolute()
                        .top(px(0.))
                        .left(px(0.))
                        .size_full()
                        .children(overlay_elements),
                )
            })
    }
}

#[cfg(test)]
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
                root.entries.is_empty(),
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
        let overlay_id = root.update(cx, |root, _cx| {
            root.add(
                Bounds::new(point(px(100.), px(100.)), size(px(200.), px(150.))),
                |_window, _cx| div().child("Overlay").into_any_element(),
            )
        });

        root.read_with(cx, |root, _| {
            assert_eq!(root.entries.len(), 1, "Should have one overlay");
            assert_eq!(root.entries[0].id, overlay_id, "Overlay ID should match");
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
        let id1 = root.update(cx, |root, _cx| {
            root.add(
                Bounds::new(point(px(0.), px(0.)), size(px(100.), px(100.))),
                |_window, _cx| div().child("Overlay 1").into_any_element(),
            )
        });

        let id2 = root.update(cx, |root, _cx| {
            root.add(
                Bounds::new(point(px(50.), px(50.)), size(px(100.), px(100.))),
                |_window, _cx| div().child("Overlay 2").into_any_element(),
            )
        });

        root.read_with(cx, |root, _| {
            assert_eq!(root.entries.len(), 2, "Should have two overlays");
            assert_ne!(id1, id2, "Overlay IDs should be unique");
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
        let overlay_id = root.update(cx, |root, _cx| {
            root.add(
                Bounds::new(point(px(100.), px(100.)), size(px(200.), px(150.))),
                |_window, _cx| div().child("Overlay").into_any_element(),
            )
        });

        // Remove it
        let removed = root.update(cx, |root, _cx| root.remove(overlay_id));
        assert!(removed, "Remove should return true for existing overlay");

        root.read_with(cx, |root, _| {
            assert!(
                root.entries.is_empty(),
                "Should have no overlays after removal"
            );
        });

        // Try to remove again (should fail)
        let removed_again = root.update(cx, |root, _cx| root.remove(overlay_id));
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
            for i in 0..5 {
                root.add(
                    Bounds::new(
                        point(px(i as f32 * 10.), px(i as f32 * 10.)),
                        size(px(50.), px(50.)),
                    ),
                    |_window, _cx| div().child(format!("Overlay {}", i)).into_any_element(),
                );
            }
        });

        root.read_with(cx, |root, _| {
            assert_eq!(root.entries.len(), 5, "Should have five overlays");
        });

        // Clear all
        root.update(cx, |root, _cx| root.clear());

        root.read_with(cx, |root, _| {
            assert!(
                root.entries.is_empty(),
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

        let expected_bounds = Bounds::new(point(px(42.), px(84.)), size(px(200.), px(300.)));

        root.update(cx, |root, _cx| {
            root.add(expected_bounds, |_window, _cx| {
                div().child("Overlay").into_any_element()
            });
        });

        root.read_with(cx, |root, _| {
            let entry = &root.entries[0];
            assert_eq!(entry.bounds.origin.x, px(42.), "X origin should match");
            assert_eq!(entry.bounds.origin.y, px(84.), "Y origin should match");
            assert_eq!(entry.bounds.size.width, px(200.), "Width should match");
            assert_eq!(entry.bounds.size.height, px(300.), "Height should match");
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

        // The window creation itself validates rendering works
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
                Bounds::new(point(px(10.), px(10.)), size(px(100.), px(100.))),
                |_window, _cx| {
                    div()
                        .id("overlay-content")
                        .child("Overlay")
                        .into_any_element()
                },
            );
            cx.notify();
        });

        let _cx = VisualTestContext::from_window(window.into(), cx);

        // The window creation itself validates rendering works
    }

    #[gpui::test]
    fn test_overlay_entry_unique_ids(_cx: &mut TestAppContext) {
        // Create multiple OverlayEntry instances and verify they have unique IDs
        let bounds = Bounds::new(point(px(0.), px(0.)), size(px(100.), px(100.)));

        let entry1 = OverlayEntry::new(bounds, |_window, _cx| div().into_any_element());
        let entry2 = OverlayEntry::new(bounds, |_window, _cx| div().into_any_element());
        let entry3 = OverlayEntry::new(bounds, |_window, _cx| div().into_any_element());

        assert_ne!(entry1.id, entry2.id, "Entry IDs should be unique");
        assert_ne!(entry2.id, entry3.id, "Entry IDs should be unique");
        assert_ne!(entry1.id, entry3.id, "Entry IDs should be unique");
    }
}
