//! Theme system providing colors, typography, and layout dimensions.
//!
//! Themes support multiple variants (e.g., dark and light modes) with a
//! consistent set of semantic color tokens and size scales.

mod schema;
pub use schema::*;

mod deserializers;

mod ext;
pub use ext::*;

mod kinds;
pub use kinds::*;
