use std::{rc::Rc, sync::Arc};

use gpui::{
    App, ElementId, Entity, FocusHandle, InteractiveElement, ParentElement, SharedString, Styled,
    Window, div, prelude::*, px,
};
use gpui_squircle::{SquircleStyled, squircle};

use crate::{
    ElementIdExt, PositionalParentElement,
    components::{
        Toggle, ToggleVariant,
        select::{Confirm, MoveDown, MoveUp, SelectItem, SelectState},
    },
    extensions::clickable::Clickable,
    primitives::{Deferrable, DeferredConfig},
    theme::{ThemeExt, ThemeLayerKind},
    utils::PixelsExt,
    views::Root,
};

#[derive(IntoElement)]
pub struct SelectMenu<V: 'static, I: SelectItem<Value = V> + 'static> {
    id: ElementId,
    layer: ThemeLayerKind,
    state: Arc<SelectState<V, I>>,
    on_item_click: Rc<dyn Fn(bool, Arc<SelectState<V, I>>, SharedString, &mut Window, &mut App)>,
    focus_handle: Option<FocusHandle>,
    deferred_config: DeferredConfig,
}

fn default_on_item_click<V: 'static, I: SelectItem<Value = V> + 'static>(
    checked: bool,
    state: Arc<SelectState<V, I>>,
    item_name: SharedString,
    _window: &mut Window,
    cx: &mut App,
) {
    if checked {
        let _ = state.select_item(cx, item_name.clone()).unwrap();
    } else {
        let _ = state.remove_selection(cx);
    }
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> SelectMenu<V, I> {
    pub fn new(id: impl Into<ElementId>, state: impl Into<Arc<SelectState<V, I>>>) -> Self {
        Self {
            id: id.into(),
            layer: ThemeLayerKind::Tertiary,
            state: state.into(),
            on_item_click: Rc::new(default_on_item_click),
            focus_handle: None,
            deferred_config: DeferredConfig::default(),
        }
    }

    pub fn layer(mut self, layer: ThemeLayerKind) -> Self {
        self.layer = layer;
        self
    }

    pub fn on_item_click(
        mut self,
        on_item_click: impl Fn(bool, Arc<SelectState<V, I>>, SharedString, &mut Window, &mut App)
        + 'static,
    ) -> Self {
        self.on_item_click = Rc::new(on_item_click);
        self
    }

    pub fn focus_handle(mut self, focus_handle: FocusHandle) -> Self {
        self.focus_handle = Some(focus_handle);
        self
    }
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> Deferrable for SelectMenu<V, I> {
    const DEFAULT_PRIORITY: usize = 1;

    fn deferred_config_mut(&mut self) -> &mut DeferredConfig {
        &mut self.deferred_config
    }

    fn deferred_config(&self) -> &DeferredConfig {
        &self.deferred_config
    }
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> RenderOnce for SelectMenu<V, I> {
    fn render(self, window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let background_color = self.layer.resolve(cx);
        let border_color = self.layer.next().resolve(cx);
        let corner_radius = cx.get_theme().layout.corner_radii.md;
        let line_height = cx.get_theme().layout.text.default_font.line_height;
        let text_size = cx.get_theme().layout.text.default_font.sizes.body.clone();
        let padding = cx.get_theme().layout.padding.md;
        let horizontal_padding = cx.get_theme().layout.padding.lg - padding;
        let vertical_padding =
            cx.get_theme()
                .layout
                .size
                .lg
                .padding_needed_for_height(window, text_size, line_height)
                - padding;

        let menu_visible_transition = self.state.menu_visible_transition.clone();
        let menu_visible_delta = menu_visible_transition.evaluate(window, cx).value();

        let state_for_up = self.state.clone();
        let state_for_down = self.state.clone();
        let state_for_confirm = self.state.clone();
        let on_item_click_for_confirm = self.on_item_click.clone();

        let focus_handle = self
            .focus_handle
            .as_ref()
            .unwrap_or_else(|| {
                window
                    .use_keyed_state(
                        self.id.with_suffix("state:focus_handle"),
                        cx,
                        |_window, cx| cx.focus_handle(),
                    )
                    .read(cx)
            })
            .clone();

        // Track whether we've synced for this menu open session
        let has_synced = window.use_keyed_state(
            self.id.with_suffix("state:has_synced"),
            cx,
            |_window, _cx| false,
        );

        // Track which item is currently hovered by mouse
        let hovered_item: Entity<Option<SharedString>> = window.use_keyed_state(
            self.id.with_suffix("state:hovered_item"),
            cx,
            |_window, _cx| None,
        );

        // Reset sync flag and hover state when menu is closed
        if menu_visible_delta == 0. && *has_synced.read(cx) {
            has_synced.update(cx, |synced, _cx| *synced = false);
            hovered_item.update(cx, |hovered, _cx| *hovered = None);
        }

        let hovered_item_for_up = hovered_item.clone();
        let hovered_item_for_down = hovered_item.clone();

        div()
            .id(self.id.clone())
            .key_context("SelectMenu")
            .track_focus(&focus_handle)
            .on_action(move |_: &MoveUp, window, cx| {
                // Clear hover state when using keyboard navigation
                hovered_item_for_up.update(cx, |hovered, _cx| *hovered = None);
                // Sync highlight to focused item before moving (handles tab navigation)
                state_for_up.move_highlight_up(window, cx);
            })
            .on_action(move |_: &MoveDown, window, cx| {
                // Clear hover state when using keyboard navigation
                hovered_item_for_down.update(cx, |hovered, _cx| *hovered = None);
                // Sync highlight to focused item before moving (handles tab navigation)
                state_for_down.move_highlight_down(window, cx);
            })
            .on_action(move |_: &Confirm, window, cx| {
                let highlighted = state_for_confirm.highlighted_item.read(cx).clone();
                if let Some(item_name) = highlighted {
                    let selected =
                        state_for_confirm.selected_item.read(cx).as_ref() == Some(&item_name);
                    (on_item_click_for_confirm)(
                        !selected,
                        state_for_confirm.clone(),
                        item_name,
                        window,
                        cx,
                    );
                }
            })
            .when(menu_visible_delta != 0., |this| {
                // We only want the click event if the menu
                // is transitioning towards the visible state.
                if menu_visible_transition.read_goal(cx) == &true.into() {
                    let root = window
                        .root::<Root>()
                        .flatten()
                        .expect("Expected gpui_tesserae::Root to be the root view!");

                    root.update(cx, |root, cx| {
                        let state = self.state.clone();

                        root.on_any_mouse_down(move |_event, _window, cx| {
                            state.hide_menu(cx);
                        });

                        cx.notify();
                    });
                }

                // Only sync highlight to selection once when menu first opens
                if !*has_synced.read(cx) {
                    self.state.sync_highlight_to_selection(cx);
                    focus_handle.focus(window, cx);
                    has_synced.update(cx, |synced, _cx| *synced = true);
                }

                let state = self.state.clone();

                // Register focus listeners for each item to sync highlighted state on tab navigation
                // Store subscriptions in keyed state to keep them alive
                let focus_handles: Vec<_> = state
                    .items
                    .read(cx)
                    .iter()
                    .map(|(_, entry)| entry.focus_handle.clone())
                    .collect();

                let _focus_subscriptions: Entity<Vec<gpui::Subscription>> = window.use_keyed_state(
                    self.id.with_suffix("state:focus_subscriptions"),
                    cx,
                    |window, cx| {
                        focus_handles
                            .iter()
                            .map(|focus_handle| {
                                let focus_handle = focus_handle.clone();

                                let state_for_focus = state.clone();
                                window.on_focus_in(&focus_handle.clone(), cx, move |_window, cx| {
                                    state_for_focus.sync_highlight_to_focused(cx, &focus_handle);
                                })
                            })
                            .collect()
                    },
                );

                let any_item_focused = state
                    .items
                    .read(cx)
                    .iter()
                    .any(|(_, entry)| entry.focus_handle.is_focused(window));

                this.opacity(menu_visible_delta)
                    .w_full()
                    .flex()
                    .flex_col()
                    .p(padding)
                    .child(
                        squircle()
                            .absolute_expand()
                            .rounded(corner_radius)
                            .bg(background_color)
                            .border_color(border_color)
                            .border(px(1.))
                            .border_inside(),
                    )
                    .children(state.items.read(cx).iter().map(|(item_name, entry)| {
                        let highlighted_item = self.state.highlighted_item.read(cx).clone();
                        let current_hovered = hovered_item.read(cx).clone();
                        let is_any_hovered = current_hovered.is_some();

                        let selected =
                            self.state.selected_item.read(cx).as_ref() == Some(item_name);
                        let is_keyboard_highlighted = highlighted_item.as_ref() == Some(item_name);
                        let is_mouse_hovered = current_hovered.as_ref() == Some(item_name);
                        let is_focused = entry.focus_handle.is_focused(window);

                        // Show highlight if:
                        // - Mouse is hovering this item, OR
                        // - This item is focused, OR
                        // - Key arrows highlighted this item AND mouse isn't hovering over
                        //   any over item AND no item is focused.
                        let show_highlight = is_mouse_hovered
                            || is_focused
                            || (is_keyboard_highlighted && !is_any_hovered && !any_item_focused);

                        let hovered_item_for_hover = hovered_item.clone();
                        let item_name_for_hover = item_name.clone();

                        div()
                            .id(self.id.with_suffix("item_row").with_suffix(item_name))
                            .w_full()
                            .flex()
                            .track_focus(&entry.focus_handle)
                            .on_mouse_down(gpui::MouseButton::Left, |_event, window, cx| {
                                window.prevent_default();
                                cx.stop_propagation();
                            })
                            .on_hover(move |is_hovered, _window, cx| {
                                hovered_item_for_hover.update(cx, |hovered, _cx| {
                                    if *is_hovered {
                                        *hovered = Some(item_name_for_hover.clone());
                                    } else if hovered.as_ref() == Some(&item_name_for_hover) {
                                        *hovered = None;
                                    }
                                });
                            })
                            .child(
                                Toggle::new(self.id.with_suffix("item").with_suffix(item_name))
                                    .checked(selected || show_highlight)
                                    .variant(if show_highlight {
                                        ToggleVariant::Tertiary
                                    } else {
                                        ToggleVariant::Secondary
                                    })
                                    .justify_start()
                                    .rounded(corner_radius - padding)
                                    .child_right(entry.item.display(window, cx))
                                    .pl(horizontal_padding)
                                    .pr(horizontal_padding)
                                    .pt(vertical_padding)
                                    .pb(vertical_padding)
                                    .w_full()
                                    .map(|this| {
                                        let state = self.state.clone();
                                        let item_name = item_name.clone();
                                        let on_item_click = self.on_item_click.clone();

                                        Clickable::on_click(this, move |_event, window, cx| {
                                            (on_item_click)(
                                                !selected,
                                                state.clone(),
                                                item_name.clone(),
                                                window,
                                                cx,
                                            )
                                        })
                                    }),
                            )
                    }))
            })
            .map(|this| self.apply_deferred(this))
    }
}
