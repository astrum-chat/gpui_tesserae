use std::collections::HashMap;

use gpui::{App, Entity, SharedString};
use thiserror::Error;

use crate::components::select::SelectItem;

pub struct SelectState<V: 'static, I: SelectItem<Value = V> + 'static> {
    pub(crate) items: Entity<SelectItemsMap<V, I>>,
    pub(crate) selected_item: Entity<Option<SharedString>>,
}

impl<V: 'static, I: SelectItem<Value = V> + 'static> SelectState<V, I> {
    pub fn new(
        items: Entity<SelectItemsMap<V, I>>,
        selected_item: Entity<Option<SharedString>>,
    ) -> Self {
        Self {
            items,
            selected_item,
        }
    }

    pub fn push_item(&self, cx: &mut App, item: I) {
        self.items.update(cx, |this, cx| {
            this.push_item(item);
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

    pub fn cancel_selection(&self, cx: &mut App) {
        self.selected_item.update(cx, |this, cx| {
            if this == &None {
                return;
            };

            *this = None;
            cx.notify();
        });
    }
}

pub struct SelectItemsMap<V: 'static, I: SelectItem<Value = V> + 'static>(HashMap<SharedString, I>);

impl<V: 'static, I: SelectItem<Value = V> + 'static> SelectItemsMap<V, I> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn push_item(&mut self, item: I) {
        self.0.insert(item.name(), item);
    }

    pub fn get(&self, item_name: &SharedString) -> Option<&I> {
        self.0.get(item_name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&SharedString, &I)> {
        self.0.iter()
    }
}

#[derive(Error, Debug)]
pub enum SelectItemError {
    #[error("An item with this name doesn't exist.")]
    InvalidName,
    #[error("The allowed amount of selected items has been reached.")]
    LimitReached,
}
