use std::time::Duration;

use gpui::Context;

const BLINK_INTERVAL: Duration = Duration::from_millis(530);

/// Manages the blinking cursor animation using an epoch-based timer to handle concurrent timers.
pub struct CursorBlink {
    visible: bool,
    epoch: usize,
}

impl CursorBlink {
    /// Creates a new cursor blink state with the cursor initially visible.
    pub fn new() -> Self {
        Self {
            visible: true,
            epoch: 0,
        }
    }

    /// Returns whether the cursor should currently be visible.
    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Resets blink to visible and restarts the timer. Call after cursor movement or text edits.
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.visible = true;
        self.epoch = self.epoch.wrapping_add(1);
        self.schedule_blink(self.epoch, cx);
    }

    /// Starts the blink cycle. Call when input gains focus.
    pub fn start(&mut self, cx: &mut Context<Self>) {
        self.visible = true;
        self.epoch = self.epoch.wrapping_add(1);
        self.schedule_blink(self.epoch, cx);
    }

    /// Stops blinking and keeps cursor visible. Call when input loses focus.
    pub fn stop(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.visible = true;
    }

    fn schedule_blink(&mut self, epoch: usize, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(BLINK_INTERVAL).await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |blink, cx| {
                    blink.blink(epoch, cx);
                });
            }
        })
        .detach();
    }

    fn blink(&mut self, epoch: usize, cx: &mut Context<Self>) {
        if epoch != self.epoch {
            return;
        }
        self.visible = !self.visible;
        cx.notify();
        self.schedule_blink(epoch, cx);
    }
}
