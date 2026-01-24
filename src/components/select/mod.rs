use std::{sync::Arc, time::Duration};

use gpui::{
    App, ElementId, InteractiveElement, IntoElement, Length, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div, ease_out_quint,
    prelude::FluentBuilder, px, radians, relative,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::Lerp;

use crate::{
    ElementIdExt, TesseraeIconKind,
    components::Icon,
    conitional_transition, conitional_transition_update,
    extensions::click_behavior::{ClickBehavior, ClickBehaviorExt},
    primitives::FocusRing,
    theme::{ThemeExt, ThemeLayerKind},
    utils::{PixelsExt, disabled_transition},
};

struct SelectStyles {
    width: Length,
}

impl Default for SelectStyles {
    fn default() -> Self {
        Self {
            width: Length::Auto,
        }
    }
}

mod menu;
pub use menu::*;

mod item;
pub use item::{SelectItem, SelectItemEntry};

mod state;
pub use state::*;

/// A dropdown select component with keyboard navigation support.
#[derive(IntoElement)]
pub struct Select<V: 'static, I: SelectItem<Value = V> + 'static> {
    id: ElementId,
    disabled: bool,
    layer: ThemeLayerKind,
    state: Arc<SelectState<V, I>>,
    click_behavior: ClickBehavior,
    style: SelectStyles,
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> Select<V, I> {
    /// Creates a new select component with the given ID and shared state.
    pub fn new(id: impl Into<ElementId>, state: impl Into<Arc<SelectState<V, I>>>) -> Self {
        let state = state.into();

        Self {
            id: id.into(),
            disabled: false,
            layer: ThemeLayerKind::Tertiary,
            state: state.into(),
            click_behavior: ClickBehavior::default(),
            style: SelectStyles::default(),
        }
    }

    /// Sets a fixed width.
    pub fn w(mut self, width: impl Into<Length>) -> Self {
        self.style.width = width.into();
        self
    }

    /// Sets width to auto, sizing based on content.
    pub fn w_auto(mut self) -> Self {
        self.style.width = Length::Auto;
        self
    }

    /// Sets width to fill the parent container.
    pub fn w_full(mut self) -> Self {
        self.style.width = relative(100.).into();
        self
    }

    /// Sets the disabled state, preventing interaction.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> ClickBehaviorExt for Select<V, I> {
    fn click_behavior_mut(&mut self) -> &mut ClickBehavior {
        &mut self.click_behavior
    }
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> RenderOnce for Select<V, I> {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let (primary_text_color, secondary_text_color) =
            cx.get_theme().variants.active(cx).colors.text.all();
        let primary_accent_color = cx.get_theme().variants.active(cx).colors.accent.primary;
        let background_color = self.layer.resolve(cx);
        let border_color = self.layer.next().resolve(cx);
        let border_hover_color = border_color.lerp(&primary_text_color, 0.07);
        let font_family = cx.get_theme().layout.text.default_font.family[0].clone();
        let line_height = cx.get_theme().layout.text.default_font.line_height;
        let text_size = /*self
            .style
            .text_size
            .unwrap_or_else(|| */cx.get_theme().layout.text.default_font.sizes.body.clone()/*)*/;
        let corner_radius = cx.get_theme().layout.corner_radii.md;
        //let corner_radii_override = self.style.corner_radii;
        //let padding_override = self.style.padding;
        // let inner_padding_override = self.style.inner_padding;
        let horizontal_padding = cx.get_theme().layout.padding.lg;
        let vertical_padding =
            cx.get_theme()
                .layout
                .size
                .lg
                .padding_needed_for_height(window, text_size, line_height);

        let is_hover_state =
            window.use_keyed_state(self.id.with_suffix("state:hover"), cx, |_cx, _window| false);
        let is_hover = *is_hover_state.read(cx);

        let focus_handle = window
            .use_keyed_state(
                self.id.with_suffix("state:focus_handle"),
                cx,
                |_window, cx| cx.focus_handle().tab_stop(true),
            )
            .read(cx)
            .clone();

        // Register this Select's focus handle with the shared state
        self.state.register_focus_handle(cx, focus_handle.clone());

        // Use contains_focused instead of is_focused so that the menu stays open
        // when focus moves to a menu item (which is a descendant of the Select).
        let is_focus = focus_handle.contains_focused(window, cx);

        let is_disabled = self.disabled;
        let disabled_transition = disabled_transition(self.id.clone(), window, cx, is_disabled);

        if is_disabled && is_focus {
            window.blur();
        }

        let border_color_transition = conitional_transition!(
            self.id.with_suffix("state:transition:border_color"),
            window,
            cx,
            Duration::from_millis(400),
            {
                is_focus => primary_accent_color,
                is_hover => border_hover_color,
                _ => border_color
            }
        )
        .with_easing(ease_out_quint());

        let menu_visible_transition = conitional_transition_update!(
            cx,
            self
                .state
                .menu_visible_transition.clone(),
            {
                self.state.any_select_focused(window, cx) => true,
                _ => false
            }
        );

        let menu_visible_delta = menu_visible_transition.evaluate(window, cx).value();

        div()
            .id(self.id.clone())
            .cursor(if is_disabled {
                gpui::CursorStyle::OperationNotAllowed
            } else {
                gpui::CursorStyle::PointingHand
            })
            .w(self.style.width)
            .h_auto()
            .pl(horizontal_padding)
            .pr(horizontal_padding)
            .pt(vertical_padding)
            .pb(vertical_padding)
            .gap(horizontal_padding)
            .flex()
            .flex_col()
            .opacity(*disabled_transition.evaluate(window, cx))
            .child(
                FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone())
                    .rounded(corner_radius),
            )
            .child(
                squircle()
                    .absolute_expand()
                    .rounded(corner_radius)
                    .bg(background_color)
                    .border(px(1.))
                    .border_inside()
                    .border_color(*border_color_transition.evaluate(window, cx)),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(text_size)
                    .text_color(secondary_text_color)
                    .font_family(font_family.clone())
                    .map(|this| {
                        let Some(item_name) = self.state.selected_item.read(cx) else {
                            return this.child("No item selected");
                        };

                        let Some(entry) = self.state.items.read(cx).get(item_name) else {
                            return this.child("No item selected");
                        };

                        this.child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .text_size(text_size)
                                .text_color(primary_text_color)
                                .font_family(font_family)
                                .child(entry.item.display(window, cx)),
                        )
                    })
                    .child(
                        Icon::new(TesseraeIconKind::ArrowDown)
                            .size(px(11.))
                            .color(secondary_text_color)
                            .map(|this| {
                                let rotation = radians(
                                    ((1. - menu_visible_delta) * 180.) * std::f32::consts::PI
                                        / 180.0,
                                );

                                this.rotate(rotation)
                            }),
                    ),
            )
            .when(menu_visible_delta != 0., |this| {
                this.child(
                    div()
                        .w_full()
                        .absolute()
                        .top_full()
                        .left_0()
                        .pt(cx.get_theme().layout.padding.md)
                        .child(
                            SelectMenu::new(self.id.with_suffix("menu"), self.state.clone())
                                .focus_handle(focus_handle.clone()),
                        ),
                )
            })
            .when(!is_disabled, |this| {
                let behavior = self.click_behavior;

                let focus_handle_on_mouse_down = focus_handle.clone();

                this.on_hover(move |hover, _window, cx| {
                    is_hover_state.update(cx, |this, cx| {
                        *this = *hover;
                        cx.notify();
                    });
                })
                .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    behavior.apply(window, cx);
                    focus_handle_on_mouse_down.focus(window, cx);
                })
                .track_focus(&focus_handle)
            })
    }
}

/// Default click handler that selects or deselects the clicked item and closes the menu.
pub fn default_on_item_click<V: 'static, I: SelectItem<Value = V> + 'static>(
    checked: bool,
    state: Arc<SelectState<V, I>>,
    item_name: SharedString,
    _window: &mut Window,
    cx: &mut App,
) {
    if checked {
        let _ = state.select_item(cx, item_name.clone());
    } else {
        state.remove_selection(cx);
    }

    state.hide_menu(cx);
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use gpui::{App, AppContext, SharedString, TestAppContext, VisualTestContext, Window};
    use gpui_transitions::{BoolLerp, TransitionState};

    /// Helper to create select state entities for tests
    fn create_test_state_entities(
        cx: &mut TestAppContext,
    ) -> (
        gpui::Entity<SelectItemsMap<String, TestSelectItem>>,
        gpui::Entity<Option<SharedString>>,
        gpui::Entity<Option<SharedString>>,
        gpui::Entity<TransitionState<BoolLerp<f32>>>,
        gpui::Entity<Vec<gpui::WeakFocusHandle>>,
    ) {
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| None::<SharedString>);
        let highlighted = cx.new(|_cx| None::<SharedString>);
        let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
        let focus_handles = cx.new(|_cx| Vec::new());
        (items, selected, highlighted, visible, focus_handles)
    }

    /// A simple test item for use in Select tests.
    #[derive(Clone)]
    struct TestSelectItem {
        name: SharedString,
        value: String,
    }

    impl TestSelectItem {
        fn new(name: impl Into<SharedString>, value: impl Into<String>) -> Self {
            Self {
                name: name.into(),
                value: value.into(),
            }
        }
    }

    impl SelectItem for TestSelectItem {
        type Value = String;

        fn name(&self) -> SharedString {
            self.name.clone()
        }

        fn value(&self) -> &Self::Value {
            &self.value
        }

        fn display(&self, _window: &mut Window, _cx: &App) -> impl IntoElement {
            gpui::div().child(self.name.clone()).into_any_element()
        }
    }

    #[gpui::test]
    fn test_select_creation(cx: &mut TestAppContext) {
        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);

        cx.update(|cx| {
            let state = SelectState::new(cx, items, selected, highlighted, visible, focus_handles);
            let select = Select::new("test-select", state);
            assert!(!select.disabled, "Select should start enabled");
        });
    }

    #[gpui::test]
    fn test_select_state_push_item(cx: &mut TestAppContext) {
        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);
        let state = cx.update(|cx| {
            SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted,
                visible,
                focus_handles,
            )
        });

        items.read_with(cx, |items, _| {
            assert!(items.iter().count() == 0, "Items should start empty");
        });

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
        });

        items.read_with(cx, |items, _| {
            assert_eq!(items.iter().count(), 1, "Should have one item");
            assert!(items.get(&"item1".into()).is_some(), "Item should exist");
        });
    }

    #[gpui::test]
    fn test_select_state_select_item(cx: &mut TestAppContext) {
        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);
        let state = cx.update(|cx| {
            SelectState::new(
                cx,
                items.clone(),
                selected.clone(),
                highlighted,
                visible,
                focus_handles,
            )
        });

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
        });

        // Select an item
        cx.update(|cx| {
            let result = state.select_item(cx, "item1");
            assert!(result.is_ok(), "Selecting existing item should succeed");
        });

        selected.read_with(cx, |selected, _| {
            assert_eq!(
                *selected,
                Some("item1".into()),
                "Selected item should be item1"
            );
        });

        // Select another item
        cx.update(|cx| {
            let result = state.select_item(cx, "item2");
            assert!(result.is_ok(), "Selecting existing item should succeed");
        });

        selected.read_with(cx, |selected, _| {
            assert_eq!(
                *selected,
                Some("item2".into()),
                "Selected item should be item2"
            );
        });
    }

    #[gpui::test]
    fn test_select_state_select_invalid_item(cx: &mut TestAppContext) {
        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);
        let state = cx.update(|cx| {
            SelectState::new(
                cx,
                items,
                selected.clone(),
                highlighted,
                visible,
                focus_handles,
            )
        });

        // Try to select non-existent item
        cx.update(|cx| {
            let result = state.select_item(cx, "nonexistent");
            assert!(result.is_err(), "Selecting non-existent item should fail");
        });

        selected.read_with(cx, |selected, _| {
            assert!(selected.is_none(), "Selection should remain empty");
        });
    }

    #[gpui::test]
    fn test_select_state_cancel_selection(cx: &mut TestAppContext) {
        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);
        let state = cx.update(|cx| {
            SelectState::new(
                cx,
                items.clone(),
                selected.clone(),
                highlighted,
                visible,
                focus_handles,
            )
        });

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            let _ = state.select_item(cx, "item1");
        });

        selected.read_with(cx, |selected, _| {
            assert!(selected.is_some(), "Should have selection");
        });

        // Removes the selection.
        cx.update(|cx| {
            state.remove_selection(cx);
        });

        selected.read_with(cx, |selected, _| {
            assert!(selected.is_none(), "Selection should be cancelled");
        });
    }

    #[gpui::test]
    fn test_select_items_map(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut items = SelectItemsMap::<String, TestSelectItem>::new();

            items.push_item(cx, TestSelectItem::new("a", "value_a"));
            items.push_item(cx, TestSelectItem::new("b", "value_b"));
            items.push_item(cx, TestSelectItem::new("c", "value_c"));

            assert_eq!(items.iter().count(), 3, "Should have 3 items");

            let entry_a = items.get(&"a".into());
            assert!(entry_a.is_some(), "Item 'a' should exist");
            assert_eq!(entry_a.unwrap().item.value(), "value_a");

            let entry_b = items.get(&"b".into());
            assert!(entry_b.is_some(), "Item 'b' should exist");
            assert_eq!(entry_b.unwrap().item.value(), "value_b");

            let item_none = items.get(&"nonexistent".into());
            assert!(item_none.is_none(), "Nonexistent item should be None");
        });
    }

    #[gpui::test]
    fn test_select_layer(cx: &mut TestAppContext) {
        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);

        cx.update(|cx| {
            let state = SelectState::new(
                cx,
                items.clone(),
                selected.clone(),
                highlighted,
                visible,
                focus_handles,
            );
            let select = Select::new("test-select", state);
            assert!(
                matches!(select.layer, ThemeLayerKind::Tertiary),
                "Select should default to tertiary layer"
            );
        });
    }

    #[gpui::test]
    fn test_select_renders_in_window(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};
        use crate::views::Root;

        let window = cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            cx.open_window(Default::default(), |window, cx| {
                let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
                let selected = cx.new(|_cx| None::<SharedString>);
                let highlighted = cx.new(|_cx| None::<SharedString>);
                let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
                let focus_handles = cx.new(|_cx| Vec::new());

                let state = Arc::new(SelectState::new(
                    cx,
                    items,
                    selected,
                    highlighted,
                    visible,
                    focus_handles,
                ));
                let test_view = cx.new(|_cx| SelectTestView { state });
                cx.new(|cx| Root::new(test_view, window, cx))
            })
            .unwrap()
        });

        let _cx = VisualTestContext::from_window(window.into(), cx);

        // The window creation itself validates rendering works
    }

    #[gpui::test]
    fn test_move_highlight_down_from_none(cx: &mut TestAppContext) {
        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| None::<SharedString>);
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            );
            (highlighted, state)
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
            state.push_item(cx, TestSelectItem::new("item3", "value3"));
        });
        highlighted.read_with(cx, |h, _| {
            assert!(h.is_none(), "Highlight should start as None");
        });

        // Move down from None should highlight first item
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_down(window, cx);
        })
        .unwrap();

        highlighted.read_with(cx, |h, _| {
            assert_eq!(*h, Some("item1".into()), "Should highlight first item");
        });
    }

    #[gpui::test]
    fn test_move_highlight_down_sequential(cx: &mut TestAppContext) {
        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| None::<SharedString>);
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            );
            (highlighted, state)
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
            state.push_item(cx, TestSelectItem::new("item3", "value3"));
        });

        // Move down through all items
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_down(window, cx);
        })
        .unwrap();
        highlighted.read_with(cx, |h, _| {
            assert_eq!(
                *h,
                Some("item1".into()),
                "First move should highlight item1"
            );
        });

        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_down(window, cx);
        })
        .unwrap();
        highlighted.read_with(cx, |h, _| {
            assert_eq!(
                *h,
                Some("item2".into()),
                "Second move should highlight item2"
            );
        });

        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_down(window, cx);
        })
        .unwrap();
        highlighted.read_with(cx, |h, _| {
            assert_eq!(
                *h,
                Some("item3".into()),
                "Third move should highlight item3"
            );
        });
    }

    #[gpui::test]
    fn test_move_highlight_down_wraps_around(cx: &mut TestAppContext) {
        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| Some("item3".into()));
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            );
            (highlighted, state)
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
            state.push_item(cx, TestSelectItem::new("item3", "value3"));
        });

        // Move down from last item should wrap to first
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_down(window, cx);
        })
        .unwrap();

        highlighted.read_with(cx, |h, _| {
            assert_eq!(*h, Some("item1".into()), "Should wrap around to first item");
        });
    }

    #[gpui::test]
    fn test_move_highlight_up_from_none(cx: &mut TestAppContext) {
        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| None::<SharedString>);
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            );
            (highlighted, state)
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
            state.push_item(cx, TestSelectItem::new("item3", "value3"));
        });

        // Move up from None should highlight last item
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_up(window, cx);
        })
        .unwrap();

        highlighted.read_with(cx, |h, _| {
            assert_eq!(*h, Some("item3".into()), "Should highlight last item");
        });
    }

    #[gpui::test]
    fn test_move_highlight_up_sequential(cx: &mut TestAppContext) {
        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| Some("item3".into()));
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            );
            (highlighted, state)
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
            state.push_item(cx, TestSelectItem::new("item3", "value3"));
        });

        // Move up through items
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_up(window, cx);
        })
        .unwrap();
        highlighted.read_with(cx, |h, _| {
            assert_eq!(
                *h,
                Some("item2".into()),
                "First move up should highlight item2"
            );
        });

        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_up(window, cx);
        })
        .unwrap();
        highlighted.read_with(cx, |h, _| {
            assert_eq!(
                *h,
                Some("item1".into()),
                "Second move up should highlight item1"
            );
        });
    }

    #[gpui::test]
    fn test_move_highlight_up_wraps_around(cx: &mut TestAppContext) {
        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| Some("item1".into()));
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            );
            (highlighted, state)
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
            state.push_item(cx, TestSelectItem::new("item3", "value3"));
        });

        // Move up from first item should wrap to last
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_up(window, cx);
        })
        .unwrap();

        highlighted.read_with(cx, |h, _| {
            assert_eq!(*h, Some("item3".into()), "Should wrap around to last item");
        });
    }

    #[gpui::test]
    fn test_move_highlight_on_empty_items(cx: &mut TestAppContext) {
        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| None::<SharedString>);
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            );
            (highlighted, state)
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        // Don't add any items - move operations should be no-ops
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_down(window, cx);
        })
        .unwrap();
        highlighted.read_with(cx, |h, _| {
            assert!(h.is_none(), "Highlight should remain None on empty list");
        });

        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_up(window, cx);
        })
        .unwrap();
        highlighted.read_with(cx, |h, _| {
            assert!(h.is_none(), "Highlight should remain None on empty list");
        });
    }

    #[gpui::test]
    fn test_confirm_highlight_selects_item(cx: &mut TestAppContext) {
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| None::<SharedString>);
        let highlighted = cx.new(|_cx| Some("item2".into()));
        let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
        let focus_handles = cx.new(|_cx| Vec::new());

        let state = cx.update(|cx| {
            Arc::new(SelectState::new(
                cx,
                items.clone(),
                selected.clone(),
                highlighted.clone(),
                visible.clone(),
                focus_handles,
            ))
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
            state.push_item(cx, TestSelectItem::new("item3", "value3"));
            state.show_menu(cx);
        });

        // Confirm the highlight
        cx.update_window(window.into(), |_view, window, cx| {
            state.confirm_highlight(window, cx);
        })
        .unwrap();

        // Selected should now be item2
        selected.read_with(cx, |s, _| {
            assert_eq!(
                *s,
                Some("item2".into()),
                "Selected should be the highlighted item"
            );
        });
    }

    #[gpui::test]
    fn test_confirm_highlight_with_no_highlight(cx: &mut TestAppContext) {
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| None::<SharedString>);
        let highlighted = cx.new(|_cx| None::<SharedString>);
        let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
        let focus_handles = cx.new(|_cx| Vec::new());

        let state = cx.update(|cx| {
            Arc::new(SelectState::new(
                cx,
                items.clone(),
                selected.clone(),
                highlighted.clone(),
                visible.clone(),
                focus_handles,
            ))
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.show_menu(cx);
        });

        // Confirm with no highlight should do nothing
        cx.update_window(window.into(), |_view, window, cx| {
            state.confirm_highlight(window, cx);
        })
        .unwrap();

        selected.read_with(cx, |s, _| {
            assert!(s.is_none(), "Selected should remain None");
        });
    }

    #[gpui::test]
    fn test_sync_highlight_to_selection(cx: &mut TestAppContext) {
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| Some("item2".into()));
        let highlighted = cx.new(|_cx| None::<SharedString>);
        let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
        let focus_handles = cx.new(|_cx| Vec::new());

        let state = cx.update(|cx| {
            SelectState::new(
                cx,
                items.clone(),
                selected.clone(),
                highlighted.clone(),
                visible,
                focus_handles,
            )
        });

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
            state.push_item(cx, TestSelectItem::new("item3", "value3"));
        });

        // Sync highlight to selection
        cx.update(|cx| {
            state.sync_highlight_to_selection(cx);
        });

        highlighted.read_with(cx, |h, _| {
            assert_eq!(
                *h,
                Some("item2".into()),
                "Highlight should sync to selected item"
            );
        });
    }

    #[gpui::test]
    fn test_sync_highlight_to_empty_selection(cx: &mut TestAppContext) {
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| None::<SharedString>);
        let highlighted = cx.new(|_cx| Some("item1".into()));
        let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
        let focus_handles = cx.new(|_cx| Vec::new());

        let state = cx.update(|cx| {
            SelectState::new(
                cx,
                items.clone(),
                selected.clone(),
                highlighted.clone(),
                visible,
                focus_handles,
            )
        });

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
        });

        // Sync highlight to empty selection
        cx.update(|cx| {
            state.sync_highlight_to_selection(cx);
        });

        highlighted.read_with(cx, |h, _| {
            assert!(h.is_none(), "Highlight should sync to None");
        });
    }

    #[gpui::test]
    fn test_items_map_preserves_insertion_order(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut items = SelectItemsMap::<String, TestSelectItem>::new();

            // Insert items in specific order
            items.push_item(cx, TestSelectItem::new("charlie", "value_c"));
            items.push_item(cx, TestSelectItem::new("alpha", "value_a"));
            items.push_item(cx, TestSelectItem::new("bravo", "value_b"));

            // Verify iteration order matches insertion order (not alphabetical)
            let names: Vec<_> = items.iter().map(|(name, _)| name.clone()).collect();
            assert_eq!(
                names,
                vec![
                    SharedString::from("charlie"),
                    SharedString::from("alpha"),
                    SharedString::from("bravo")
                ],
                "Items should iterate in insertion order"
            );
        });
    }

    #[gpui::test]
    fn test_items_map_index_methods(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let mut items = SelectItemsMap::<String, TestSelectItem>::new();

            items.push_item(cx, TestSelectItem::new("first", "value1"));
            items.push_item(cx, TestSelectItem::new("second", "value2"));
            items.push_item(cx, TestSelectItem::new("third", "value3"));

            assert_eq!(items.len(), 3, "Should have 3 items");
            assert!(!items.is_empty(), "Should not be empty");
            assert_eq!(
                items.get_index_of(&"first".into()),
                Some(0),
                "first should be at index 0"
            );
            assert_eq!(
                items.get_index_of(&"second".into()),
                Some(1),
                "second should be at index 1"
            );
            assert_eq!(
                items.get_index_of(&"third".into()),
                Some(2),
                "third should be at index 2"
            );
            assert_eq!(
                items.get_index_of(&"nonexistent".into()),
                None,
                "nonexistent should return None"
            );
            assert_eq!(
                items.get_index(0).map(|(n, _)| n.clone()),
                Some("first".into()),
                "Index 0 should be first"
            );
            assert_eq!(
                items.get_index(1).map(|(n, _)| n.clone()),
                Some("second".into()),
                "Index 1 should be second"
            );
            assert_eq!(
                items.get_index(2).map(|(n, _)| n.clone()),
                Some("third".into()),
                "Index 2 should be third"
            );
            assert!(
                items.get_index(3).is_none(),
                "Index 3 should be out of bounds"
            );
            assert_eq!(
                items.first().map(|(n, _)| n.clone()),
                Some("first".into()),
                "First item should be 'first'"
            );
            assert_eq!(
                items.last().map(|(n, _)| n.clone()),
                Some("third".into()),
                "Last item should be 'third'"
            );
        });
    }

    #[gpui::test]
    fn test_items_map_empty_index_methods(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let items = SelectItemsMap::<String, TestSelectItem>::new();

            assert_eq!(items.len(), 0, "Empty map should have length 0");
            assert!(items.is_empty(), "Empty map should be empty");
            assert!(items.first().is_none(), "First on empty should be None");
            assert!(items.last().is_none(), "Last on empty should be None");
            assert!(
                items.get_index(0).is_none(),
                "get_index on empty should be None"
            );
        });
    }

    #[gpui::test]
    fn test_move_highlight_with_invalid_current_highlight(cx: &mut TestAppContext) {
        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            // Set highlighted to an item that doesn't exist
            let highlighted = cx.new(|_cx| Some("nonexistent".into()));
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            );
            (highlighted, state)
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
        });

        // Move down with invalid highlight should go to first item
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_down(window, cx);
        })
        .unwrap();

        highlighted.read_with(cx, |h, _| {
            assert_eq!(
                *h,
                Some("item1".into()),
                "Should reset to first item when current highlight is invalid"
            );
        });
    }

    #[gpui::test]
    fn test_move_highlight_up_with_invalid_current_highlight(cx: &mut TestAppContext) {
        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            // Set highlighted to an item that doesn't exist
            let highlighted = cx.new(|_cx| Some("nonexistent".into()));
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            );
            (highlighted, state)
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            state.push_item(cx, TestSelectItem::new("item2", "value2"));
        });

        // Move up with invalid highlight should go to first item
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_up(window, cx);
        })
        .unwrap();

        highlighted.read_with(cx, |h, _| {
            assert_eq!(
                *h,
                Some("item1".into()),
                "Should reset to first item when current highlight is invalid"
            );
        });
    }

    #[gpui::test]
    fn test_full_keyboard_navigation_flow(cx: &mut TestAppContext) {
        let (selected, highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| None::<SharedString>);
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = Arc::new(SelectState::new(
                cx,
                items.clone(),
                selected.clone(),
                highlighted.clone(),
                visible.clone(),
                focus_handles,
            ));
            (selected, highlighted, state)
        });

        let window = cx
            .update(|cx| cx.open_window(Default::default(), |_window, cx| cx.new(|_| gpui::Empty)))
            .unwrap();

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("apple", "fruit1"));
            state.push_item(cx, TestSelectItem::new("banana", "fruit2"));
            state.push_item(cx, TestSelectItem::new("cherry", "fruit3"));
        });

        // Simulate: Open menu, navigate down twice, confirm selection
        cx.update(|cx| {
            state.show_menu(cx);
            state.sync_highlight_to_selection(cx); // Usually done when menu opens
        });

        // Navigate down to first item
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_down(window, cx);
        })
        .unwrap();
        highlighted.read_with(cx, |h, _| {
            assert_eq!(*h, Some("apple".into()));
        });

        // Navigate down to second item
        cx.update_window(window.into(), |_view, window, cx| {
            state.move_highlight_down(window, cx);
        })
        .unwrap();
        highlighted.read_with(cx, |h, _| {
            assert_eq!(*h, Some("banana".into()));
        });

        // Confirm selection
        cx.update_window(window.into(), |_view, window, cx| {
            state.confirm_highlight(window, cx);
        })
        .unwrap();

        selected.read_with(cx, |s, _| {
            assert_eq!(*s, Some("banana".into()), "Should have selected banana");
        });
    }

    #[gpui::test]
    fn test_select_click_behavior_default(cx: &mut TestAppContext) {
        use crate::extensions::click_behavior::ClickBehaviorExt;

        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);

        cx.update(|cx| {
            let state = SelectState::new(cx, items, selected, highlighted, visible, focus_handles);
            let mut select = Select::new("test-select", state);
            let behavior = select.click_behavior_mut();

            assert!(
                !behavior.allow_propagation,
                "Select should not allow propagation by default"
            );
            assert!(
                !behavior.allow_default,
                "Select should not allow default by default"
            );
        });
    }

    #[gpui::test]
    fn test_select_allow_click_propagation(cx: &mut TestAppContext) {
        use crate::extensions::click_behavior::ClickBehaviorExt;

        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);

        cx.update(|cx| {
            let state = SelectState::new(cx, items, selected, highlighted, visible, focus_handles);
            let mut select = Select::new("test-select", state).allow_click_propagation();
            let behavior = select.click_behavior_mut();

            assert!(
                behavior.allow_propagation,
                "Select should allow propagation after calling allow_click_propagation"
            );
            assert!(
                !behavior.allow_default,
                "Select should still not allow default"
            );
        });
    }

    #[gpui::test]
    fn test_select_allow_default_click_behaviour(cx: &mut TestAppContext) {
        use crate::extensions::click_behavior::ClickBehaviorExt;

        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);

        cx.update(|cx| {
            let state = SelectState::new(cx, items, selected, highlighted, visible, focus_handles);
            let mut select = Select::new("test-select", state).allow_default_click_behaviour();
            let behavior = select.click_behavior_mut();

            assert!(
                !behavior.allow_propagation,
                "Select should still not allow propagation"
            );
            assert!(
                behavior.allow_default,
                "Select should allow default after calling allow_default_click_behaviour"
            );
        });
    }

    #[gpui::test]
    fn test_select_click_behavior_chain(cx: &mut TestAppContext) {
        use crate::extensions::click_behavior::ClickBehaviorExt;

        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);

        cx.update(|cx| {
            let state = SelectState::new(cx, items, selected, highlighted, visible, focus_handles);
            let mut select = Select::new("test-select", state)
                .allow_click_propagation()
                .allow_default_click_behaviour();
            let behavior = select.click_behavior_mut();

            assert!(
                behavior.allow_propagation,
                "Select should allow propagation"
            );
            assert!(behavior.allow_default, "Select should allow default");
        });
    }

    /// Test view that contains a Select
    struct SelectTestView {
        state: Arc<SelectState<String, TestSelectItem>>,
    }

    impl gpui::Render for SelectTestView {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            gpui::div()
                .size_full()
                .child(Select::new("test-select", self.state.clone()))
        }
    }

    /// Test view that contains a SelectMenu for action dispatch testing
    struct SelectMenuTestView {
        state: Arc<SelectState<String, TestSelectItem>>,
    }

    impl gpui::Render for SelectMenuTestView {
        fn render(
            &mut self,
            _window: &mut gpui::Window,
            _cx: &mut gpui::Context<Self>,
        ) -> impl IntoElement {
            gpui::div()
                .size_full()
                .child(SelectMenu::new("test-menu", self.state.clone()))
        }
    }

    #[gpui::test]
    fn test_arrow_down_action_moves_highlight(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};
        use crate::views::Root;

        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            super::state::init(cx);
        });

        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| None::<SharedString>);
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = Arc::new(SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            ));
            (highlighted, state)
        });

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("first", "value1"));
            state.push_item(cx, TestSelectItem::new("second", "value2"));
            state.push_item(cx, TestSelectItem::new("third", "value3"));
            state.show_menu(cx);
        });

        let window = cx
            .update(|cx| {
                cx.open_window(Default::default(), |window, cx| {
                    let test_view = cx.new(|_cx| SelectMenuTestView {
                        state: state.clone(),
                    });
                    cx.new(|cx| Root::new(test_view, window, cx))
                })
            })
            .unwrap();

        let mut vcx = VisualTestContext::from_window(window.into(), cx);

        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveDown), cx);
        });

        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(
                *h,
                Some("first".into()),
                "First arrow down should highlight first item"
            );
        });

        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveDown), cx);
        });

        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(
                *h,
                Some("second".into()),
                "Second arrow down should highlight second item"
            );
        });

        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveDown), cx);
        });

        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(
                *h,
                Some("third".into()),
                "Third arrow down should highlight third item"
            );
        });

        // Should wrap to first
        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveDown), cx);
        });

        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(
                *h,
                Some("first".into()),
                "Fourth arrow down should wrap to first item"
            );
        });
    }

    #[gpui::test]
    fn test_arrow_up_action_moves_highlight(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};
        use crate::views::Root;

        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            super::state::init(cx);
        });

        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| None::<SharedString>);
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = Arc::new(SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            ));
            (highlighted, state)
        });

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("first", "value1"));
            state.push_item(cx, TestSelectItem::new("second", "value2"));
            state.push_item(cx, TestSelectItem::new("third", "value3"));
            state.show_menu(cx);
        });

        let window = cx
            .update(|cx| {
                cx.open_window(Default::default(), |window, cx| {
                    let test_view = cx.new(|_cx| SelectMenuTestView {
                        state: state.clone(),
                    });
                    cx.new(|cx| Root::new(test_view, window, cx))
                })
            })
            .unwrap();

        let mut vcx = VisualTestContext::from_window(window.into(), cx);

        // Should go to last item
        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveUp), cx);
        });

        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(
                *h,
                Some("third".into()),
                "First arrow up should highlight last item"
            );
        });

        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveUp), cx);
        });

        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(
                *h,
                Some("second".into()),
                "Second arrow up should highlight second item"
            );
        });

        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveUp), cx);
        });

        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(
                *h,
                Some("first".into()),
                "Third arrow up should highlight first item"
            );
        });

        // Should wrap to last
        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveUp), cx);
        });

        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(
                *h,
                Some("third".into()),
                "Fourth arrow up should wrap to last item"
            );
        });
    }

    #[gpui::test]
    fn test_arrow_keys_with_existing_selection(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};
        use crate::views::Root;

        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            super::state::init(cx);
        });

        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| Some("second".into())); // Pre-select second item
            let highlighted = cx.new(|_cx| None::<SharedString>);
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = Arc::new(SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            ));
            (highlighted, state)
        });

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("first", "value1"));
            state.push_item(cx, TestSelectItem::new("second", "value2"));
            state.push_item(cx, TestSelectItem::new("third", "value3"));
            // This should sync highlight to selection
            state.show_menu(cx);
        });

        let window = cx
            .update(|cx| {
                cx.open_window(Default::default(), |window, cx| {
                    let test_view = cx.new(|_cx| SelectMenuTestView {
                        state: state.clone(),
                    });
                    cx.new(|cx| Root::new(test_view, window, cx))
                })
            })
            .unwrap();

        let mut vcx = VisualTestContext::from_window(window.into(), cx);

        // Force a draw to trigger sync_highlight_to_selection
        vcx.run_until_parked();

        // Highlight should now be synced to selection (second)
        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(
                *h,
                Some("second".into()),
                "Highlight should sync to selected item on menu open"
            );
        });

        // Now arrow down should go to third
        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveDown), cx);
        });

        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(
                *h,
                Some("third".into()),
                "Arrow down from second should go to third"
            );
        });
    }

    #[gpui::test]
    fn test_confirm_action_selects_highlighted_item(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};
        use crate::views::Root;

        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            super::state::init(cx);
        });

        let (selected, highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| None::<SharedString>);
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = Arc::new(SelectState::new(
                cx,
                items.clone(),
                selected.clone(),
                highlighted.clone(),
                visible,
                focus_handles,
            ));
            (selected, highlighted, state)
        });

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("apple", "fruit1"));
            state.push_item(cx, TestSelectItem::new("banana", "fruit2"));
            state.push_item(cx, TestSelectItem::new("cherry", "fruit3"));
            state.show_menu(cx);
        });

        let window = cx
            .update(|cx| {
                cx.open_window(Default::default(), |window, cx| {
                    let test_view = cx.new(|_cx| SelectMenuTestView {
                        state: state.clone(),
                    });
                    cx.new(|cx| Root::new(test_view, window, cx))
                })
            })
            .unwrap();

        let mut vcx = VisualTestContext::from_window(window.into(), cx);

        // Navigate down twice to highlight "banana"
        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveDown), cx);
        });
        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(MoveDown), cx);
        });

        highlighted.read_with(&vcx, |h, _| {
            assert_eq!(*h, Some("banana".into()), "Should highlight banana");
        });

        // Dispatch Confirm action to select the highlighted item
        vcx.update(|window, cx| {
            window.dispatch_action(Box::new(Confirm), cx);
        });

        selected.read_with(&vcx, |s, _| {
            assert_eq!(
                *s,
                Some("banana".into()),
                "Confirm should select the highlighted item"
            );
        });
    }

    #[gpui::test]
    fn test_highlight_persists_across_multiple_arrow_presses(cx: &mut TestAppContext) {
        use crate::theme::{Theme, ThemeExt};
        use crate::views::Root;

        cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);
            super::state::init(cx);
        });

        let (highlighted, state) = cx.update(|cx| {
            let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
            let selected = cx.new(|_cx| None::<SharedString>);
            let highlighted = cx.new(|_cx| None::<SharedString>);
            let visible = cx.new(|_cx| TransitionState::new(BoolLerp::truthy()));
            let focus_handles = cx.new(|_cx| Vec::new());

            let state = Arc::new(SelectState::new(
                cx,
                items.clone(),
                selected,
                highlighted.clone(),
                visible,
                focus_handles,
            ));
            (highlighted, state)
        });

        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("one", "1"));
            state.push_item(cx, TestSelectItem::new("two", "2"));
            state.push_item(cx, TestSelectItem::new("three", "3"));
            state.push_item(cx, TestSelectItem::new("four", "4"));
            state.push_item(cx, TestSelectItem::new("five", "5"));
            state.show_menu(cx);
        });

        let window = cx
            .update(|cx| {
                cx.open_window(Default::default(), |window, cx| {
                    let test_view = cx.new(|_cx| SelectMenuTestView {
                        state: state.clone(),
                    });
                    cx.new(|cx| Root::new(test_view, window, cx))
                })
            })
            .unwrap();

        let mut vcx = VisualTestContext::from_window(window.into(), cx);

        // Press down 5 times, checking each step
        let expected_sequence = ["one", "two", "three", "four", "five"];
        for (i, expected) in expected_sequence.iter().enumerate() {
            vcx.update(|window, cx| {
                window.dispatch_action(Box::new(MoveDown), cx);
            });

            highlighted.read_with(&vcx, |h, _| {
                assert_eq!(
                    *h,
                    Some(SharedString::from(*expected)),
                    "After {} arrow down presses, should highlight '{}'",
                    i + 1,
                    expected
                );
            });
        }

        // Now go back up
        let expected_up_sequence = ["four", "three", "two", "one"];
        for (i, expected) in expected_up_sequence.iter().enumerate() {
            vcx.update(|window, cx| {
                window.dispatch_action(Box::new(MoveUp), cx);
            });

            highlighted.read_with(&vcx, |h, _| {
                assert_eq!(
                    *h,
                    Some(SharedString::from(*expected)),
                    "After {} arrow up presses, should highlight '{}'",
                    i + 1,
                    expected
                );
            });
        }
    }

    #[gpui::test]
    fn test_stale_focus_handles_are_cleaned_up(cx: &mut TestAppContext) {
        cx.update(|cx| {
            super::state::init(cx);
        });

        let (items, selected, highlighted, visible, focus_handles) = create_test_state_entities(cx);

        let state = cx.update(|cx| {
            Arc::new(SelectState::new(
                cx,
                items,
                selected,
                highlighted,
                visible,
                focus_handles,
            ))
        });

        // Register some focus handles and verify they persist while alive
        let (handle1, handle2, handle3) = cx.update(|cx| {
            let handle1 = cx.focus_handle();
            let handle2 = cx.focus_handle();
            let handle3 = cx.focus_handle();

            state.register_focus_handle(cx, handle1.clone());
            state.register_focus_handle(cx, handle2.clone());
            state.register_focus_handle(cx, handle3.clone());

            // Verify we have 3 handles registered while they're still alive
            assert_eq!(
                state.select_focus_handles.read(cx).len(),
                3,
                "Should have 3 focus handles registered"
            );

            (handle1, handle2, handle3)
        });

        // Drop one handle and verify cleanup happens on next register
        drop(handle1);

        cx.update(|cx| {
            let handle4 = cx.focus_handle();
            state.register_focus_handle(cx, handle4.clone());

            // Should have 3 handles: handle2, handle3, handle4 (handle1 was cleaned up)
            assert_eq!(
                state.select_focus_handles.read(cx).len(),
                3,
                "Stale handle should have been cleaned up"
            );

            drop(handle4);
        });

        // Keep handles alive until end of test
        drop(handle2);
        drop(handle3);
    }
}
