use gpui::{App, Window};

/// Settings that control click event behavior.
///
/// By default, click handlers call `cx.stop_propagation()` and `window.prevent_default()`.
/// This struct allows components to opt out of this behavior.
#[derive(Clone, Copy, Default)]
pub struct ClickBehavior {
    /// If true, `cx.stop_propagation()` will NOT be called.
    pub allow_propagation: bool,
    /// If true, `window.prevent_default()` will NOT be called.
    pub allow_default: bool,
}

impl ClickBehavior {
    /// Applies the click behavior settings to the given window and app context.
    ///
    /// Calls `window.prevent_default()` unless `allow_default` is true,
    /// and calls `cx.stop_propagation()` unless `allow_propagation` is true.
    pub fn apply(&self, window: &mut Window, cx: &mut App) {
        if !self.allow_default {
            window.prevent_default();
        }
        if !self.allow_propagation {
            cx.stop_propagation();
        }
    }
}

/// A trait for components that support controlling click event behavior.
///
/// Implement this trait to allow users to opt out of automatic `stop_propagation()`
/// and `prevent_default()` calls in click handlers.
pub trait ClickBehaviorExt: Sized {
    /// Returns a mutable reference to the click behavior settings.
    fn click_behavior_mut(&mut self) -> &mut ClickBehavior;

    /// Allows the click event to propagate to parent elements.
    ///
    /// By default, click handlers call `cx.stop_propagation()`. Calling this method
    /// will prevent that behavior, allowing the event to bubble up to parent handlers.
    fn allow_click_propagation(mut self) -> Self {
        self.click_behavior_mut().allow_propagation = true;
        self
    }

    /// Allows the browser/system default behavior for the click event.
    ///
    /// By default, click handlers call `window.prevent_default()`. Calling this method
    /// will prevent that behavior, allowing the default action to occur.
    fn allow_default_click_behaviour(mut self) -> Self {
        self.click_behavior_mut().allow_default = true;
        self
    }
}
