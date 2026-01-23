use gpui::Context;
use std::time::Duration;

const BLINK_INTERVAL: Duration = Duration::from_millis(530);

pub struct CursorBlink {
    visible: bool,
    epoch: usize,
}

impl CursorBlink {
    pub fn new() -> Self {
        Self {
            visible: true,
            epoch: 0,
        }
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    /// Call this when the cursor moves or text is edited to reset the blink state
    pub fn reset(&mut self, cx: &mut Context<Self>) {
        self.visible = true;
        self.epoch = self.epoch.wrapping_add(1);
        self.schedule_blink(self.epoch, cx);
    }

    /// Start the blink timer
    pub fn start(&mut self, cx: &mut Context<Self>) {
        self.visible = true;
        self.epoch = self.epoch.wrapping_add(1);
        self.schedule_blink(self.epoch, cx);
    }

    /// Stop blinking
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
