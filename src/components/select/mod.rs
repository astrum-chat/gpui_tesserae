use std::{sync::Arc, time::Duration};

use gpui::{
    ElementId, InteractiveElement, IntoElement, MouseButton, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, div, ease_out_quint, prelude::FluentBuilder, px, radians,
};
use gpui_squircle::{SquircleStyled, squircle};
use gpui_transitions::Lerp;

use crate::{
    ElementIdExt, TesseraeIconKind,
    components::Icon,
    conitional_transition,
    primitives::FocusRing,
    theme::{ThemeExt, ThemeLayerKind},
    utils::{PixelsExt, disabled_transition},
};

mod menu;
pub use menu::*;

mod item;
pub use item::*;

mod state;
pub use state::*;

#[derive(IntoElement)]
pub struct Select<V: 'static, I: SelectItem<Value = V> + 'static> {
    id: ElementId,
    disabled: bool,
    layer: ThemeLayerKind,
    state: Arc<SelectState<V, I>>,
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> Select<V, I> {
    pub fn new(id: impl Into<ElementId>, state: impl Into<Arc<SelectState<V, I>>>) -> Self {
        Self {
            id: id.into(),
            disabled: false,
            layer: ThemeLayerKind::Tertiary,
            state: state.into(),
        }
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
        let is_focus = focus_handle.is_focused(window);

        let is_disabled = self.disabled;
        let disabled_transition = disabled_transition(self.id.clone(), window, cx, is_disabled);

        if is_focus && is_disabled {
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

        let menu_visible_transition = conitional_transition!(
            self.id.with_suffix("state:transition:menu_visible"),
            window,
            cx,
            Duration::from_millis(350),
            {
                is_focus => 1.,
                _ => 0.
            }
        )
        .with_easing(ease_out_quint());

        let menu_visible_delta = *menu_visible_transition.evaluate(window, cx);

        div()
            .id(self.id.clone())
            .track_focus(&focus_handle)
            .cursor_pointer()
            .w_full()
            .h_auto()
            .pl(horizontal_padding)
            .pr(horizontal_padding)
            .pt(vertical_padding)
            .pb(vertical_padding)
            .gap(horizontal_padding)
            .flex()
            .flex_col()
            .map(|this| {
                let focus_handle = focus_handle.clone();
                let disabled_delta = *disabled_transition.evaluate(window, cx);

                this.opacity(disabled_delta).child(
                    FocusRing::new(self.id.with_suffix("focus_ring"), focus_handle.clone())
                        .rounded(corner_radius),
                )
            })
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

                        let Some(item) = self.state.items.read(cx).get(item_name) else {
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
                                .child(item.display(window, cx)),
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
                                .opacity(menu_visible_delta),
                        ),
                )
            })
            .when(!is_disabled, |this| {
                this.on_hover(move |hover, _window, cx| {
                    is_hover_state.update(cx, |this, _cx| *this = *hover);
                    cx.notify(is_hover_state.entity_id());
                })
                .on_mouse_down(MouseButton::Left, move |_event, window, cx| {
                    // We want to disable the default focus / blur behaviour.
                    window.prevent_default();
                    focus_handle.focus(window, cx);
                })
            })
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use gpui::{App, AppContext, SharedString, TestAppContext, VisualTestContext, Window};

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
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| None::<SharedString>);

        cx.update(|_cx| {
            let state = SelectState::new(items, selected);
            let select = Select::new("test-select", state);
            assert!(!select.disabled, "Select should start enabled");
        });
    }

    #[gpui::test]
    fn test_select_state_push_item(cx: &mut TestAppContext) {
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| None::<SharedString>);
        let state = SelectState::new(items.clone(), selected);

        // Initially empty
        items.read_with(cx, |items, _| {
            assert!(items.iter().count() == 0, "Items should start empty");
        });

        // Add an item
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
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| None::<SharedString>);
        let state = SelectState::new(items.clone(), selected.clone());

        // Add items first
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
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| None::<SharedString>);
        let state = SelectState::new(items, selected.clone());

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
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| None::<SharedString>);
        let state = SelectState::new(items.clone(), selected.clone());

        // Add and select an item
        cx.update(|cx| {
            state.push_item(cx, TestSelectItem::new("item1", "value1"));
            let _ = state.select_item(cx, "item1");
        });

        selected.read_with(cx, |selected, _| {
            assert!(selected.is_some(), "Should have selection");
        });

        // Cancel the selection
        cx.update(|cx| {
            state.cancel_selection(cx);
        });

        selected.read_with(cx, |selected, _| {
            assert!(selected.is_none(), "Selection should be cancelled");
        });
    }

    #[gpui::test]
    fn test_select_items_map(cx: &mut TestAppContext) {
        cx.update(|_cx| {
            let mut items = SelectItemsMap::<String, TestSelectItem>::new();

            items.push_item(TestSelectItem::new("a", "value_a"));
            items.push_item(TestSelectItem::new("b", "value_b"));
            items.push_item(TestSelectItem::new("c", "value_c"));

            assert_eq!(items.iter().count(), 3, "Should have 3 items");

            let item_a = items.get(&"a".into());
            assert!(item_a.is_some(), "Item 'a' should exist");
            assert_eq!(item_a.unwrap().value(), "value_a");

            let item_b = items.get(&"b".into());
            assert!(item_b.is_some(), "Item 'b' should exist");
            assert_eq!(item_b.unwrap().value(), "value_b");

            let item_none = items.get(&"nonexistent".into());
            assert!(item_none.is_none(), "Nonexistent item should be None");
        });
    }

    #[gpui::test]
    fn test_select_layer(cx: &mut TestAppContext) {
        let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
        let selected = cx.new(|_cx| None::<SharedString>);

        cx.update(|_cx| {
            let state = SelectState::new(items.clone(), selected.clone());
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

        let window = cx.update(|cx| {
            cx.set_theme(Theme::DEFAULT);

            cx.open_window(Default::default(), |_window, cx| {
                let items = cx.new(|_cx| SelectItemsMap::<String, TestSelectItem>::new());
                let selected = cx.new(|_cx| None::<SharedString>);

                cx.new(|_cx| SelectTestView {
                    state: Arc::new(SelectState::new(items, selected)),
                })
            })
            .unwrap()
        });

        let _cx = VisualTestContext::from_window(window.into(), cx);

        // The window creation itself validates rendering works
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
}
