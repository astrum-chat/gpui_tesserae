# gpui_primitives

Headless, unstyled UI primitives for [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui).

> **Work in Progress**: This crate is under active development. Currently only contains a text input primitive.

## Overview

`gpui_primitives` provides low-level, unstyled UI building blocks that handle complex interaction logic without imposing any visual design. These primitives are designed to be wrapped by higher-level component libraries that add styling and theming.

## Available Primitives

<details>
<summary><strong>Input</strong></summary>

A text input primitive with support for:

- Single-line and multi-line modes
- Word wrapping
- Keyboard navigation
- Placeholder text
- Text transformation

### Basic Usage

```rust
use gpui::prelude::*;
use gpui_primitives::input::{Input, InputState};

fn my_input(cx: &mut App) -> impl IntoElement {
    let state = cx.new(|cx| InputState::new(cx));

    Input::new("my-input", state)
        .placeholder("Enter text...")
}
```

### Multiline Modes

The input supports several multiline configurations:

```rust
// Single-line (default).
Input::new("input", state)

// Fixed number of visible lines (scrolls after 5 lines).
Input::new("input", state)
    .line_clamp(5)

// Unlimited lines (grows indefinitely).
Input::new("input", state)
    .multiline()

// With word wrapping enabled.
Input::new("input", state)
    .multiline()
    // Word wrapping only works with multiline.
    .word_wrap(true)

// Shift+Enter for newlines (useful for form inputs where Enter submits).
Input::new("input", state)
    .multiline()
    .newline_on_shift_enter(true)
```

### Text Transformation

There are two ways to transform text, each with different behavior:

#### `transform_text` - Display-only transformation

Transforms how text is *displayed* without modifying the actual stored value. Useful for masking sensitive input.

It operates on each character to ensure the transformed text is the same length as the base text.

```rust
// Password field - displays asterisks but stores actual characters.
Input::new("password", state)
    .transform_text(|_| '*')

// The stored value remains "secret123"
// The displayed value shows "*********"
```

#### `map_text` - Value transformation

Transforms the *actual stored value* whenever text changes. Useful for enforcing input formats.

```rust
// Force uppercase - modifies the stored value.
Input::new("code", state)
    .map_text(|text| text.to_uppercase().into())

// Strips whitespace.
Input::new("username", state)
    .map_text(|text| text.replace(" ", "").into())
```

**Key difference**: `transform_text` is purely visual (the original value is preserved), while `map_text` changes the underlying text value.

</details>
