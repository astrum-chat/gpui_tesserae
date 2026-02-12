# gpui_primitives

Headless, unstyled UI primitives for [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui).

## Overview

`gpui_primitives` provides low-level, unstyled UI building blocks that handle complex interaction logic without imposing any visual design. These primitives are designed to be wrapped by higher-level component libraries that add styling and theming.

Both primitives implement GPUI's `Styled` trait, giving you access to all standard layout and styling methods (`w()`, `h()`, `bg()`, `text_color()`, `text_size()`, `line_height()`, `font_family()`, `p()`, etc.).

## Available Primitives

<details>
<summary><strong>Input</strong>: editable text input</summary>

A text input primitive with support for single-line and multiline modes, word wrapping, text selection with rounded corners, keyboard navigation, clipboard, undo/redo, IME/accent menus, placeholder text, and text transformation.

### Setup

Register default key bindings once at app startup:

```rust
use gpui_primitives::input;

fn main() {
    Application::new().run(|cx: &mut App| {
        input::init(cx);
        // ...
    });
}
```

### Basic Usage

```rust
use gpui_primitives::input::{Input, InputState};

// Create state (typically stored in your view struct)
let state = cx.new(|cx| InputState::new(cx));

// Build the input element (in your render method)
Input::new("my-input", state)
    .placeholder("Enter text...")
```

### Multiline Modes

```rust
// Single-line (default)
Input::new("input", state)

// Enables mulitline with a maximum amount
// of visible lines (scrolls after 5 lines).
Input::new("input", state)
    .multiline_clamp(5)

// Shorthand for enabling multiline with
// no maximum amount of visible lines.
Input::new("input", state)
    .multiline()

// Word wrapping (requires multiline)
Input::new("input", state)
    .multiline()
    .multiline_wrapped()

// Enter now triggers on_submit (Use Shift+Enter for newlines)
Input::new("input", state)
    .multiline()
    .multiline_wrapped()
    .on_submit(|_window, _cx| { /* handle submit */ })
```

### Text Transformation
There are two ways to transform text, each with different behavior:
```rust
use gpui_primitives::input::text_transforms;

// transform_text: display-only, stored value unchanged
Input::new("password", state)
    .transform_text(text_transforms::password)

// map_text: Modifies stored value on every change
Input::new("code", state)
    .map_text(|text| text.to_uppercase().into())
```

`transform_text` is purely visual (per-character, preserves stored value). `map_text` transforms the actual stored value.

### Selection Styling

```rust
Input::new("input", state)
    .selection_color(hsla(0.6, 0.5, 0.5, 0.3))
    .selection_rounded(px(4.))
    .selection_rounded_smoothing(0.6) // squircle effect, 0.0–1.0
```

### State API

```rust
// Set initial value (builder pattern, only if unset)
let state = cx.new(|cx| InputState::new(cx).initial_value("Hello"));

// Read current value
let text = state.read(cx).value();

// Clear and retrieve value
state.update(cx, |this, _cx| { this.clear(); });
```

### Builder Methods

| Method | Description |
|---|---|
| `placeholder(text)` | Placeholder text shown when empty |
| `multiline_clamp(n)` | Max visible lines before scrolling |
| `multiline()` | Unlimited lines with vertical scrolling |
| `multiline_wrapped()` | Enable word wrapping (requires multiline) |
| `on_submit(callback)` | Callback on Enter; forces Shift+Enter for newlines |
| `submit_disabled(bool)` | Disable the submit action |
| `secondary_newline()` | Force Shift+Enter for newlines without an on_submit callback |
| `transform_text(fn)` | Display-only per-character transform |
| `map_text(fn)` | Transform stored value on change |
| `selection_color(color)` | Selection highlight color |
| `selection_rounded(px)` | Selection corner radius |
| `selection_rounded_smoothing(f32)` | Squircle smoothing (0.0–1.0) |
| `placeholder_text_color(color)` | Placeholder text color |
| `disabled(bool)` | Disable focus and editing |
| `max_history(cx, n)` | Max undo/redo entries (default 200) |
| `debug_interior_corners(bool)` | Visualize interior selection corners |

</details>

<details>
<summary><strong>SelectableText</strong>: read-only text with selection</summary>

A read-only text component with mouse and keyboard text selection, copy support, word wrapping, and rounded selection corners.

### Setup

Register default key bindings once at app startup:

```rust
use gpui_primitives::selectable_text;

fn main() {
    Application::new().run(|cx: &mut App| {
        selectable_text::init(cx);
        // ...
    });
}
```

### Basic Usage

```rust
use gpui_primitives::selectable_text::{SelectableText, SelectableTextState};

let state = cx.new(|cx| {
    let mut s = SelectableTextState::new(cx);
    s.text("Hello, world!");
    s
});

SelectableText::new("my-text", state)
    .text_color(rgb(0xcdd6f4))
    .text_size(px(16.))
    .selection_rounded(px(4.))
```

### Multiline Modes

```rust
// Single-line (default)
SelectableText::new("text", state)

// Unlimited multiline with word wrapping
SelectableText::new("text", state)
    .multiline()
    .multiline_wrapped()

// Limit to 5 visible lines with scrolling
SelectableText::new("text", state)
    .multiline_clamp(5)
```

### Builder Methods

| Method | Description |
|---|---|
| `multiline()` | Enable unlimited multiline display |
| `multiline_clamp(n)` | Max visible lines before scrolling |
| `multiline_wrapped()` | Enable word wrapping (requires multiline) |
| `selection_color(color)` | Selection highlight color |
| `selection_rounded(px)` | Selection corner radius |
| `selection_rounded_smoothing(f32)` | Squircle smoothing (0.0–1.0) |
| `debug_wrapping(bool)` | Visualize text wrapping width |
| `debug_character_bounds(bool)` | Visualize individual character bounds |
| `debug_interior_corners(bool)` | Visualize interior selection corners |

</details>

## Examples

```bash
# Input examples (single-line, multiline, wrapped, password, etc.)
cargo run -p gpui_primitives --example input

# Selectable text with streaming updates
cargo run -p gpui_primitives --example selectable_text
```
