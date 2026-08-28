use gpui::{Context, MouseMoveEvent, Window};

use super::CellarApp;
use cellar_desktop_gpui::theme::ui_scale;

const LEFT_MIN: f32 = 200.;
const LEFT_MAX: f32 = 600.;
const RIGHT_MIN: f32 = 280.;
const RIGHT_MAX: f32 = 720.;
const BOTTOM_MIN: f32 = 140.;

impl CellarApp {
    pub(super) fn reveal_bottom_panel(&mut self, quarter_height: bool, cx: &mut Context<Self>) {
        if self.bottom_panel_open {
            return;
        }
        self.bottom_panel_open = true;
        if quarter_height {
            let height = f32::from(self.window_bounds.size.height) / ui_scale();
            self.bottom_panel_height = bottom_target(height);
        }
        cx.notify();
    }

    pub(super) fn resize_panels(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let scale = ui_scale();
        let x = f32::from(event.position.x);
        let y = f32::from(event.position.y);
        let mut changed = false;
        if let Some((start, width)) = self.sidebar_resize {
            self.sidebar_width = (width + (x - start) / scale).clamp(LEFT_MIN, LEFT_MAX);
            changed = true;
        }
        if let Some((start, width)) = self.right_panel_resize {
            self.right_panel_width = (width - (x - start) / scale).clamp(RIGHT_MIN, RIGHT_MAX);
            changed = true;
        }
        if let Some((start, height)) = self.bottom_panel_resize {
            let max = (f32::from(self.window_bounds.size.height) / scale * 0.7)
                .round()
                .max(BOTTOM_MIN);
            self.bottom_panel_height = (height - (y - start) / scale).clamp(BOTTOM_MIN, max);
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    pub(super) fn finish_panel_resize(&mut self, cx: &mut Context<Self>) {
        if self.sidebar_resize.take().is_some()
            || self.right_panel_resize.take().is_some()
            || self.bottom_panel_resize.take().is_some()
        {
            cx.notify();
        }
    }
}

fn bottom_target(window_height: f32) -> f32 {
    let max = (window_height * 0.7).round().max(BOTTOM_MIN);
    (window_height / 4.).round().clamp(BOTTOM_MIN, max)
}

#[cfg(test)]
mod tests {
    use super::{bottom_target, BOTTOM_MIN, LEFT_MAX, LEFT_MIN, RIGHT_MAX, RIGHT_MIN};

    #[test]
    fn panel_limits_match_the_classic_shell() {
        assert_eq!((LEFT_MIN, LEFT_MAX), (200., 600.));
        assert_eq!((RIGHT_MIN, RIGHT_MAX), (280., 720.));
        assert_eq!(BOTTOM_MIN, 140.);
        assert_eq!(bottom_target(800.), 200.);
        assert_eq!(bottom_target(480.), 140.);
    }
}
