//! A UI component library built on GPUI providing reusable, themed components.
//!
//! Tesserae (meaning mosaic tiles) offers building blocks for constructing
//! user interfaces with consistent styling, smooth transitions, and
//! interactive behaviors.

#![warn(missing_docs)]

/// Low-level UI building blocks like focus rings and layout wrappers.
pub mod primitives;

/// Traits for adding interactive behaviors to components.
pub mod extensions;

/// High-level view components for application structure.
pub mod views;

/// Ready-to-use UI components like buttons, inputs, and selects.
pub mod components;

/// Theming system for consistent styling across components.
pub mod theme;

mod utils;
pub use utils::{ElementIdExt, PositionalParentElement};

mod assets;
pub use assets::*;

mod init;
pub use init::*;
