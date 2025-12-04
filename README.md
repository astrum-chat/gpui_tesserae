# Tesserae

Tesserae is a WIP component system for [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui).

It currently offers the following components:
- Checkbox

    ![checkbox](/readme/assets/checkbox.png)

- Switch

    ![switch](/readme/assets/switch.png)

- Single-line Input

    ![input](/readme/assets/input.png)

- Button

    ![button](/readme/assets/button.png)

# Setup
```rs
use gpui::{App, Application, prelude::*};
use gpui_tesserae::{TesseraeAssets, assets};

fn main() {
    Application::new()
        .with_assets(
            // Tesserae provides an `assets!` macro which
            // makes it easy to compose multiple asset providers
            // together.
            assets![TesseraeAssets],
        )
        .run(|cx: &mut App| {
            /// Tesserae needs to be initialized before it can be used.
            gpui_tesserae::init(cx);
        });
}

```
