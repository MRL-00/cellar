use gpui::{px, IsZero, Pixels, Point, ScrollDelta, ScrollHandle, ScrollWheelEvent};

/// Applies dx to `handle` only for a horizontal pan or Shift+wheel.
/// Vertical-intent (including jittery trackpad dy) leaves the offset unchanged.
///
/// GPUI 0.2.2 `overflow_x_scroll` remaps unused-axis dy onto x inside
/// `paint_scroll_listener`, which runs in bubble before `on_scroll_wheel`.
pub(super) fn apply_horizontal_wheel(
    handle: &ScrollHandle,
    event: &ScrollWheelEvent,
    line_height: Pixels,
) -> bool {
    let dx = horizontal_wheel_delta(event.delta.pixel_delta(line_height), event.modifiers.shift);
    if dx.is_zero() {
        return false;
    }
    let mut offset = handle.offset();
    offset.x += dx;
    handle.set_offset(offset);
    true
}

fn horizontal_wheel_delta(delta: Point<Pixels>, shift: bool) -> Pixels {
    if f32::from(delta.x).abs() > f32::from(delta.y).abs() {
        delta.x
    } else if shift {
        delta.y
    } else {
        px(0.)
    }
}

#[cfg(test)]
mod tests {
    use super::{apply_horizontal_wheel, horizontal_wheel_delta};
    use gpui::{point, px, Modifiers, ScrollDelta, ScrollHandle, ScrollWheelEvent, TouchPhase};

    fn wheel(delta: ScrollDelta, shift: bool) -> ScrollWheelEvent {
        ScrollWheelEvent {
            position: point(px(0.), px(0.)),
            delta,
            modifiers: Modifiers {
                shift,
                ..Modifiers::default()
            },
            touch_phase: TouchPhase::Moved,
        }
    }

    #[test]
    fn vertical_pixel_wheel_does_not_pan_columns() {
        let handle = ScrollHandle::new();
        assert!(!apply_horizontal_wheel(
            &handle,
            &wheel(ScrollDelta::Pixels(point(px(0.), px(40.))), false),
            px(16.),
        ));
        assert_eq!(handle.offset().x, px(0.));
    }

    #[test]
    fn jittery_trackpad_vertical_does_not_pan_columns() {
        let handle = ScrollHandle::new();
        assert!(!apply_horizontal_wheel(
            &handle,
            &wheel(ScrollDelta::Pixels(point(px(4.), px(40.))), false),
            px(16.),
        ));
        assert_eq!(handle.offset().x, px(0.));
    }

    #[test]
    fn equal_axes_count_as_vertical() {
        assert_eq!(
            horizontal_wheel_delta(point(px(10.), px(10.)), false),
            px(0.)
        );
    }

    #[test]
    fn horizontal_pixel_pan_moves_columns() {
        let handle = ScrollHandle::new();
        assert!(apply_horizontal_wheel(
            &handle,
            &wheel(ScrollDelta::Pixels(point(px(40.), px(4.))), false),
            px(16.),
        ));
        assert_eq!(handle.offset().x, px(40.));
    }

    #[test]
    fn line_deltas_use_pixel_magnitude() {
        let line_height = px(16.);
        let handle = ScrollHandle::new();
        assert!(!apply_horizontal_wheel(
            &handle,
            &wheel(ScrollDelta::Lines(point(0., 3.)), false),
            line_height,
        ));
        assert_eq!(handle.offset().x, px(0.));

        assert!(apply_horizontal_wheel(
            &handle,
            &wheel(ScrollDelta::Lines(point(3., 0.)), false),
            line_height,
        ));
        assert_eq!(handle.offset().x, px(48.));
    }

    #[test]
    fn shift_wheel_pans_horizontally() {
        let handle = ScrollHandle::new();
        assert!(apply_horizontal_wheel(
            &handle,
            &wheel(ScrollDelta::Pixels(point(px(0.), px(24.))), true),
            px(16.),
        ));
        assert_eq!(handle.offset().x, px(24.));
    }

    #[test]
    fn gpui_overflow_x_remap_would_pan_on_vertical_wheels() {
        let vertical = point(px(0.), px(40.));
        let remapped = if f32::from(vertical.x).abs() > 0. {
            vertical.x
        } else {
            vertical.y
        };
        assert_eq!(remapped, px(40.));
        assert_eq!(horizontal_wheel_delta(vertical, false), px(0.));
    }
}
