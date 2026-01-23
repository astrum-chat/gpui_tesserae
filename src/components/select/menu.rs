use std::sync::Arc;

use gpui::{
    ElementId, Entity, FocusHandle, InteractiveElement, Length, ParentElement, SharedString,
    Styled, Subscription, WeakFocusHandle, Window, div, prelude::*, px, relative,
};
use gpui_squircle::{SquircleStyled, squircle};

use crate::{
    ElementIdExt, PositionalParentElement,
    components::{
        Toggle, ToggleVariant,
        select::{Confirm, MoveDown, MoveUp, SelectItem, SelectState},
    },
    extensions::{
        clickable::Clickable,
        deferrable::{Deferrable, DeferredConfig},
    },
    theme::{ThemeExt, ThemeLayerKind},
    utils::PixelsExt,
    views::Root,
};

struct SelectMenuStyles {
    width: Length,
}

impl Default for SelectMenuStyles {
    fn default() -> Self {
        Self {
            width: Length::Auto,
        }
    }
}

#[derive(IntoElement)]
pub struct SelectMenu<V: 'static, I: SelectItem<Value = V> + 'static> {
    id: ElementId,
    layer: ThemeLayerKind,
    state: Arc<SelectState<V, I>>,
    focus_handle: Option<FocusHandle>,
    deferred_config: DeferredConfig,
    style: SelectMenuStyles,
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> SelectMenu<V, I> {
    pub fn new(id: impl Into<ElementId>, state: impl Into<Arc<SelectState<V, I>>>) -> Self {
        Self {
            id: id.into(),
            layer: ThemeLayerKind::Tertiary,
            state: state.into(),
            focus_handle: None,
            deferred_config: DeferredConfig::default(),
            style: SelectMenuStyles::default(),
        }
    }

    pub fn w(mut self, width: impl Into<Length>) -> Self {
        self.style.width = width.into();
        self
    }

    pub fn w_auto(mut self) -> Self {
        self.style.width = Length::Auto;
        self
    }

    pub fn w_full(mut self) -> Self {
        self.style.width = relative(100.).into();
        self
    }

    pub fn layer(mut self, layer: ThemeLayerKind) -> Self {
        self.layer = layer;
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
                state_for_confirm.confirm_highlight(window, cx);
                window.blur();
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

                        root.on_any_mouse_down(move |_event, window, cx| {
                            if !state.any_select_focused(window, cx) {
                                state.hide_menu(cx);
                            }
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

                // Manage focus subscriptions via entities that update incrementally
                let item_focus_subs: Entity<ItemFocusSubscriptions> = window.use_keyed_state(
                    self.id.with_suffix("state:item_focus_subs"),
                    cx,
                    |_window, _cx| ItemFocusSubscriptions::default(),
                );
                item_focus_subs.update(cx, |subs, cx| subs.sync(&state, window, cx));

                let select_focus_subs: Entity<SelectFocusSubscriptions> = window.use_keyed_state(
                    self.id.with_suffix("state:select_focus_subs"),
                    cx,
                    |_window, _cx| SelectFocusSubscriptions::default(),
                );
                select_focus_subs.update(cx, |subs, cx| subs.sync(&state, window, cx));

                this.opacity(menu_visible_delta)
                    .w(self.style.width)
                    .flex()
                    .flex_col()
                    .gap(px(1.))
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
                        let highlighted_item = self.state.highlighted_item.read(cx).as_ref();
                        let hovered_item_exists = hovered_item.read(cx).is_some();

                        let selected =
                            self.state.selected_item.read(cx).as_ref() == Some(item_name);

                        let show_highlight =
                            !hovered_item_exists && highlighted_item == Some(item_name);

                        let hovered_item_for_hover = hovered_item.clone();
                        let item_name_for_hover = item_name.clone();

                        div()
                            .id(self.id.with_suffix("item_row").with_suffix(item_name))
                            .w_full()
                            .flex()
                            .track_focus(&entry.focus_handle)
                            .child(
                                Toggle::new(self.id.with_suffix("item").with_suffix(item_name))
                                    .w_full()
                                    .checked(selected)
                                    .variant(if selected {
                                        ToggleVariant::Secondary
                                    } else {
                                        ToggleVariant::Tertiary
                                    })
                                    .force_hover(show_highlight)
                                    .justify_start()
                                    .rounded(corner_radius - padding)
                                    .child_right(entry.item.display(window, cx))
                                    .pl(horizontal_padding)
                                    .pr(horizontal_padding)
                                    .pt(vertical_padding)
                                    .pb(vertical_padding)
                                    .on_any_mouse_down(|_event, window, _cx| {
                                        window.prevent_default();
                                    })
                                    .on_hover(move |is_hovered, _window, cx| {
                                        hovered_item_for_hover.update(cx, |this, cx| {
                                            if *is_hovered {
                                                *this = Some(item_name_for_hover.clone());
                                            } else if this.as_ref() == Some(&item_name_for_hover) {
                                                *this = None;
                                            }
                                            cx.notify();
                                        });
                                    })
                                    .map(|this| {
                                        let state = self.state.clone();
                                        let item_name = item_name.clone();

                                        this.on_click(move |_event, window, cx| {
                                            (state.on_item_click)(
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

/// Manages focus subscriptions for menu items, syncing highlighted state on tab navigation.
#[derive(Default)]
struct ItemFocusSubscriptions {
    subscriptions: Vec<(WeakFocusHandle, Subscription)>,
}

impl ItemFocusSubscriptions {
    fn sync<V: 'static, I: SelectItem<Value = V> + 'static>(
        &mut self,
        state: &Arc<SelectState<V, I>>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        let current_handles: Vec<_> = state
            .items
            .read(cx)
            .iter()
            .map(|(_, entry)| entry.focus_handle.clone())
            .collect();

        // Remove subscriptions for focus handles that no longer exist
        self.subscriptions
            .retain(|(weak, _)| weak.upgrade().is_some());

        // Add subscriptions for new focus handles
        for focus_handle in current_handles {
            let weak_focus_handle = focus_handle.downgrade();

            if self
                .subscriptions
                .iter()
                .any(|(weak, _)| weak == &focus_handle)
            {
                continue;
            }

            let state_clone = state.clone();
            let focus_handle_clone = focus_handle.clone();

            let subscription = window.on_focus_in(&focus_handle, cx, move |_window, cx| {
                state_clone.sync_highlight_to_focused(cx, &focus_handle_clone);
            });

            self.subscriptions
                .push((weak_focus_handle.clone(), subscription));
        }
    }
}

/// Manages focus subscriptions for Select components, resetting highlight when refocused.
#[derive(Default)]
struct SelectFocusSubscriptions {
    subscriptions: Vec<(WeakFocusHandle, Subscription)>,
}

impl SelectFocusSubscriptions {
    fn sync<V: 'static, I: SelectItem<Value = V> + 'static>(
        &mut self,
        state: &Arc<SelectState<V, I>>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        let current_handles: Vec<_> = state
            .select_focus_handles
            .read(cx)
            .iter()
            .filter_map(|weak| weak.upgrade())
            .collect();

        // Remove subscriptions for focus handles that no longer exist
        self.subscriptions
            .retain(|(weak, _)| weak.upgrade().is_some());

        // Add subscriptions for new focus handles
        let highlighted_item = state.highlighted_item.clone();

        for focus_handle in current_handles {
            let downgraded = focus_handle.downgrade();

            if self
                .subscriptions
                .iter()
                .any(|(weak, _)| weak == &downgraded)
            {
                continue;
            }

            let highlighted_item = highlighted_item.clone();
            let subscription = window.on_focus_in(&focus_handle, cx, move |_window, cx| {
                highlighted_item.update(cx, |this, cx| {
                    if this.is_some() {
                        *this = None;
                        cx.notify();
                    }
                });
            });

            self.subscriptions.push((downgraded, subscription));
        }
    }
}
