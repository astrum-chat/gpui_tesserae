use std::{rc::Rc, sync::Arc, time::Duration};

use gpui::{
    App, AppContext, Context, ElementId, Entity, FocusHandle, KeyBinding, SharedString,
    WeakFocusHandle, Window, actions, ease_out_quint,
};
use gpui_transitions::{BoolLerp, Transition, TransitionState};
use indexmap::IndexMap;
use thiserror::Error;

use crate::{
    ElementIdExt,
    components::select::{SelectItem, SelectItemEntry, default_on_item_click},
};

actions!(select_menu, [MoveUp, MoveDown, Confirm]);

pub type OnItemClickFn<V, I> =
    Rc<dyn Fn(bool, Arc<SelectState<V, I>>, SharedString, &mut Window, &mut App)>;

pub struct SelectState<V: 'static, I: SelectItem<Value = V> + 'static> {
    pub(crate) items: Entity<SelectItemsMap<V, I>>,
    pub(crate) selected_item: Entity<Option<SharedString>>,
    pub(crate) highlighted_item: Entity<Option<SharedString>>,
    pub menu_visible_transition: Transition<BoolLerp<f32>>,
    pub(crate) on_item_click: OnItemClickFn<V, I>,
    /// Weak focus handles from all Select components using this state.
    /// Used to determine if any associated Select has focus.
    /// Stored as weak references so stale handles can be cleaned up.
    pub(crate) select_focus_handles: Entity<Vec<WeakFocusHandle>>,
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> SelectState<V, I> {
    pub fn new(
        cx: &mut App,
        items: Entity<SelectItemsMap<V, I>>,
        selected_item: Entity<Option<SharedString>>,
        highlighted_item: Entity<Option<SharedString>>,
        menu_visible: Entity<TransitionState<BoolLerp<f32>>>,
        select_focus_handles: Entity<Vec<WeakFocusHandle>>,
    ) -> Self {
        let state = Self {
            items,
            selected_item,
            highlighted_item,
            menu_visible_transition: Transition::new(menu_visible, Duration::from_millis(275))
                .with_easing(ease_out_quint()),
            on_item_click: Rc::new(default_on_item_click),
            select_focus_handles,
        };

        state.cleanup_stale_focus_handles(cx);

        state
    }

    pub fn from_window(
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
        create_items: impl FnOnce(
            &mut Window,
            &mut Context<SelectItemsMap<V, I>>,
        ) -> SelectItemsMap<V, I>,
    ) -> Self {
        let id = id.into();

        let state = Self {
            items: window.use_keyed_state(id.with_suffix("state:items"), cx, create_items),
            selected_item: window.use_keyed_state(
                id.with_suffix("state:selected_item"),
                cx,
                |_window, _cx| None,
            ),
            highlighted_item: window.use_keyed_state(
                id.with_suffix("state:highlighted_item"),
                cx,
                |_window, _cx| None,
            ),
            menu_visible_transition: Transition::new(
                window.use_keyed_state(id.with_suffix("state:menu_visible"), cx, |_window, _cx| {
                    TransitionState::new(BoolLerp::falsey())
                }),
                Duration::from_millis(275),
            )
            .with_easing(ease_out_quint()),
            on_item_click: Rc::new(default_on_item_click),
            select_focus_handles: window.use_keyed_state(
                id.with_suffix("state:focus_handles"),
                cx,
                |_window, _cx| vec![],
            ),
        };

        state.cleanup_stale_focus_handles(cx);

        state
    }

    pub fn from_cx(cx: &mut App, items: SelectItemsMap<V, I>) -> Self {
        let state = Self {
            items: cx.new(|_cx| items),
            selected_item: cx.new(|_cx| None),
            highlighted_item: cx.new(|_cx| None),
            menu_visible_transition: Transition::new(
                cx.new(|_cx| TransitionState::new(BoolLerp::falsey())),
                Duration::from_millis(275),
            )
            .with_easing(ease_out_quint()),
            on_item_click: Rc::new(default_on_item_click),
            select_focus_handles: cx.new(|_cx| vec![]),
        };

        state.cleanup_stale_focus_handles(cx);

        state
    }

    pub fn on_item_click(
        &mut self,
        on_item_click: impl Fn(bool, Arc<SelectState<V, I>>, SharedString, &mut Window, &mut App)
        + 'static,
    ) {
        self.on_item_click = Rc::new(on_item_click);
    }

    /// Removes focus handles that are no longer valid (i.e., their associated
    /// component has been removed).
    pub(crate) fn cleanup_stale_focus_handles(&self, cx: &mut App) {
        self.select_focus_handles.update(cx, |handles, _cx| {
            let mut i = 0;
            while i < handles.len() {
                if handles[i].upgrade().is_none() {
                    handles.swap_remove(i);
                } else {
                    i += 1;
                }
            }
        });
    }

    /// Registers a focus handle from a Select component using this state.
    /// Called automatically when a Select component renders.
    pub(crate) fn register_focus_handle(&self, cx: &mut App, focus_handle: FocusHandle) {
        self.select_focus_handles.update(cx, |handles, _cx| {
            let weak = focus_handle.downgrade();

            // Clean up stale handles
            let mut i = 0;
            while i < handles.len() {
                if handles[i].upgrade().is_none() {
                    handles.swap_remove(i);
                } else {
                    i += 1;
                }
            }

            // Only add if not already present
            if !handles.contains(&weak) {
                handles.push(weak);
            }
        });
    }

    /// Checks if any Select component using this state has focus.
    pub fn any_select_focused(&self, window: &Window, cx: &mut App) -> bool {
        self.select_focus_handles
            .read(cx)
            .iter()
            .filter_map(|handle| handle.upgrade())
            .any(|handle| handle.contains_focused(window, cx))
    }

    pub fn push_item(&self, cx: &mut App, item: impl Into<I>) {
        self.items.update(cx, |this, cx| {
            this.push_item(cx, item);
            cx.notify()
        });
    }

    pub fn select_item<'a>(
        &'a self,
        cx: &'a mut App,
        item_name: impl Into<SharedString>,
    ) -> Result<(), SelectItemError> {
        let item_name = item_name.into();

        let _item = self
            .items
            .read(cx)
            .get(&item_name)
            .ok_or_else(|| SelectItemError::InvalidName)?;

        self.selected_item.update(cx, |this, cx| {
            if this.as_ref() == Some(&item_name) {
                return;
            };

            *this = Some(item_name);
            cx.notify();
        });

        Ok(())
    }

    pub fn remove_selection(&self, cx: &mut App) {
        self.selected_item.update(cx, |this, cx| {
            if this == &None {
                return;
            };

            *this = None;
            cx.notify();
        });
    }

    pub fn toggle_menu(&self, cx: &mut App) {
        self.menu_visible_transition.update(cx, |this, cx| {
            *this = this.toggle();
            cx.notify();
        });
    }

    pub fn hide_menu(&self, cx: &mut App) {
        self.menu_visible_transition.update(cx, |this, cx| {
            if this.value() == 0. {
                return;
            }

            *this = false.into();
            cx.notify();
        });
    }

    pub fn show_menu(&self, cx: &mut App) {
        self.menu_visible_transition.update(cx, |this, cx| {
            if this.value() == 1. {
                return;
            }

            *this = true.into();
            cx.notify();
        });
    }

    pub fn move_highlight_up(&self, window: &mut Window, cx: &mut App) {
        let items = self.items.read(cx);
        if items.is_empty() {
            return;
        }

        let current = self.highlighted_item.read(cx).clone();
        let new_index = match current {
            None => Some(items.len() - 1),
            Some(ref name) => {
                let current_index = items.get_index_of(name);
                match current_index {
                    Some(0) => Some(items.len() - 1),
                    Some(idx) => Some(idx - 1),
                    None => Some(0),
                }
            }
        };

        let target = new_index.and_then(|idx| {
            items
                .get_index(idx)
                .map(|(name, entry)| (name.clone(), entry.focus_handle.clone()))
        });

        if let Some((new_highlight, focus_handle)) = target {
            focus_handle.focus(window, cx);

            self.highlighted_item.update(cx, |this, cx| {
                *this = Some(new_highlight);
                cx.notify();
            });
        }
    }

    pub fn move_highlight_down(&self, window: &mut Window, cx: &mut App) {
        let items = self.items.read(cx);
        if items.is_empty() {
            return;
        }

        let current = self.highlighted_item.read(cx).clone();
        let len = items.len();
        let new_index = match current {
            None => Some(0),
            Some(ref name) => {
                let current_index = items.get_index_of(name);
                match current_index {
                    Some(idx) if idx + 1 >= len => Some(0),
                    Some(idx) => Some(idx + 1),
                    None => Some(0),
                }
            }
        };

        let target = new_index.and_then(|idx| {
            items
                .get_index(idx)
                .map(|(name, entry)| (name.clone(), entry.focus_handle.clone()))
        });

        if let Some((new_highlight, focus_handle)) = target {
            focus_handle.focus(window, cx);

            self.highlighted_item.update(cx, |this, cx| {
                *this = Some(new_highlight);
                cx.notify();
            });
        }
    }

    pub fn confirm_highlight(self: &Arc<Self>, window: &mut Window, cx: &mut App) {
        let highlighted = self.highlighted_item.read(cx).clone();
        if let Some(item_name) = highlighted {
            let selected = self.selected_item.read(cx).as_ref() == Some(&item_name);
            (self.on_item_click)(!selected, self.clone(), item_name, window, cx);
        }
    }

    pub fn sync_highlight_to_selection(&self, cx: &mut App) {
        let items = self.items.read(cx);
        let selected = self.selected_item.read(cx).clone();

        // If the selected item exists in items, use it; otherwise default to None
        let new_highlight = selected.filter(|name| items.get(name).is_some());

        self.highlighted_item.update(cx, |this, cx| {
            if *this != new_highlight {
                *this = new_highlight;
                cx.notify();
            }
        });
    }

    pub fn sync_highlight_to_focused(&self, cx: &mut App, focus_handle: &FocusHandle) {
        let items = self.items.read(cx);

        // Find the item whose focus handle is currently focused.
        let Some(item_name) = items
            .iter()
            .find(|(_, entry)| &entry.focus_handle == focus_handle)
            .map(|(name, _)| name)
        else {
            return;
        };

        let item_name = item_name.clone();

        self.highlighted_item.update(cx, |this, cx| {
            if this.as_ref() != Some(&item_name) {
                *this = Some(item_name.clone());
                cx.notify();
            }
        });
    }
}

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("up", MoveUp, Some("SelectMenu")),
        KeyBinding::new("down", MoveDown, Some("SelectMenu")),
        KeyBinding::new("enter", Confirm, Some("SelectMenu")),
    ]);
}

pub struct SelectItemsMap<V: 'static, I: SelectItem<Value = V> + 'static>(
    IndexMap<SharedString, SelectItemEntry<I>>,
);

impl<V: 'static, I: SelectItem<Value = V> + 'static> SelectItemsMap<V, I> {
    pub fn new() -> Self {
        Self(IndexMap::new())
    }

    pub fn push_item(&mut self, cx: &mut App, item: impl Into<I>) {
        let entry = SelectItemEntry::new(item.into(), cx);
        self.0.insert(entry.item.name(), entry);
    }

    pub fn get(&self, item_name: &SharedString) -> Option<&SelectItemEntry<I>> {
        self.0.get(item_name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&SharedString, &SelectItemEntry<I>)> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get_index_of(&self, item_name: &SharedString) -> Option<usize> {
        self.0.get_index_of(item_name)
    }

    pub fn get_index(&self, index: usize) -> Option<(&SharedString, &SelectItemEntry<I>)> {
        self.0.get_index(index)
    }

    pub fn first(&self) -> Option<(&SharedString, &SelectItemEntry<I>)> {
        self.0.first()
    }

    pub fn last(&self) -> Option<(&SharedString, &SelectItemEntry<I>)> {
        self.0.last()
    }
}

#[derive(Error, Debug)]
pub enum SelectItemError {
    #[error("An item with this name doesn't exist.")]
    InvalidName,
    #[error("The allowed amount of selected items has been reached.")]
    LimitReached,
}
