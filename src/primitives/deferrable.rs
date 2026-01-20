use gpui::{AnyElement, IntoElement, deferred};

/// Configuration for deferred rendering.
#[derive(Clone, Copy, Debug)]
pub struct DeferredConfig {
    /// Whether deferred rendering is enabled.
    pub enabled: bool,
    /// The priority for deferred rendering. Higher priority elements are painted later.
    pub priority: Option<usize>,
}

impl Default for DeferredConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            priority: None,
        }
    }
}

impl DeferredConfig {
    /// Creates a new config with deferring enabled and no custom priority.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            priority: None,
        }
    }

    /// Creates a new config with deferring disabled.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            priority: None,
        }
    }

    /// Creates a new config with deferring enabled and a custom priority.
    pub fn priority(priority: usize) -> Self {
        Self {
            enabled: true,
            priority: Some(priority),
        }
    }
}

/// A trait for components that support deferred rendering.
///
/// Deferred rendering allows elements to be painted after their siblings,
/// which is useful for overlays, dropdowns, and popups that need to appear
/// above other content.
pub trait Deferrable: Sized {
    /// The default priority used when deferring is enabled but no custom priority is set.
    const DEFAULT_PRIORITY: usize = 0;

    /// Returns a reference to the deferred configuration.
    fn deferred_config(&self) -> &DeferredConfig;

    /// Returns a mutable reference to the deferred configuration.
    fn deferred_config_mut(&mut self) -> &mut DeferredConfig;

    /// Enables or disables deferred rendering.
    fn deferred(mut self, enabled: bool) -> Self {
        self.deferred_config_mut().enabled = enabled;
        self
    }

    /// Wraps an element with deferred rendering based on the current configuration.
    fn apply_deferred(&self, element: impl IntoElement) -> AnyElement
    where
        Self: Sized,
    {
        let config = self.deferred_config();
        if config.enabled {
            let priority = config.priority.unwrap_or(Self::DEFAULT_PRIORITY);
            deferred(element).priority(priority).into_any_element()
        } else {
            element.into_any_element()
        }
    }
}
