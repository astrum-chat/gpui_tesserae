# Tesserae

Tesserae is a WIP component system for [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui).

It currently offers the following components:
- Checkbox

    ![checkbox](/assets/readme/checkbox.png)

- Switch

    ![switch](/assets/readme/switch.png)

- Button

    ![button](/assets/readme/button.png)

- Single-line Input

    ![input](/assets/readme/input.png)

- Select

    ![select](/assets/readme/select.png)

- Chat bubble

    ![chat bubble](/assets/readme/chat_bubble.png)

Examples can be found [here](https://github.com/astrum-chat/gpui_tesserae/tree/main/examples).

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
