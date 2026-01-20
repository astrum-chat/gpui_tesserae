use std::time::Duration;

use gpui::{App, Entity, KeyBinding, SharedString, Window, actions, ease_out_quint};
use gpui_transitions::{BoolLerp, Transition, TransitionState};
use indexmap::IndexMap;
use thiserror::Error;

use crate::components::select::{SelectItem, SelectItemEntry};

actions!(select_menu, [MoveUp, MoveDown, Confirm]);

pub struct SelectState<V: 'static, I: SelectItem<Value = V> + 'static> {
    pub items: Entity<SelectItemsMap<V, I>>,
    pub selected_item: Entity<Option<SharedString>>,
    pub highlighted_item: Entity<Option<SharedString>>,
    pub menu_visible_transition: Transition<BoolLerp<f32>>,
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> SelectState<V, I> {
    pub fn new(
        items: Entity<SelectItemsMap<V, I>>,
        selected_item: Entity<Option<SharedString>>,
        highlighted_item: Entity<Option<SharedString>>,
        menu_visible_state: Entity<TransitionState<BoolLerp<f32>>>,
    ) -> Self {
        Self {
            items,
            selected_item,
            highlighted_item,
            menu_visible_transition: Transition::new(
                menu_visible_state,
                Duration::from_millis(275),
            )
            .with_easing(ease_out_quint()),
        }
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

    pub fn confirm_highlight(&self, cx: &mut App) {
        let highlighted = self.highlighted_item.read(cx).clone();
        if let Some(item_name) = highlighted {
            let _ = self.select_item(cx, item_name);
            self.hide_menu(cx);
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
