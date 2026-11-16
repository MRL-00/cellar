use gpui::{div, prelude::*, AnyElement, Context};
use gpui_component::Icon;

use super::CellarApp;

use cellar_desktop_gpui::theme::{ui_px, ACCENT, BORDER, FG_MUTED, PANEL, PANEL_RAISED};

pub(super) fn query_ai_strip(cx: &mut Context<CellarApp>) -> AnyElement {
    div()
        .id("query-ai-strip")
        .cursor_pointer()
        .on_click(cx.listener(|this, _, window, cx| {
            this.right_panel_open = true;
            this.ai
                .draft
                .update(cx, |draft, cx| draft.focus(window, cx));
            cx.notify();
        }))
        .absolute()
        .left(ui_px(12.))
        .right(ui_px(12.))
        .bottom(ui_px(12.))
        .flex()
        .items_center()
        .justify_between()
        .rounded(ui_px(5.))
        .border_1()
        .border_dashed()
        .border_color(BORDER)
        .bg(gpui::Rgba {
            a: 0.9,
            ..PANEL.rgba()
        })
        .px(ui_px(10.))
        .py(ui_px(6.))
        .text_size(ui_px(12.5))
        .text_color(FG_MUTED)
        .child(
            div()
                .flex()
                .items_center()
                .gap(ui_px(6.))
                .child(
                    Icon::empty()
                        .path("icons/sparkles.svg")
                        .size(ui_px(11.))
                        .text_color(ACCENT),
                )
                .child("Ask AI to edit or extend this query…"),
        )
        .child(
            div()
                .flex()
                .gap(ui_px(2.))
                .child(query_keycap("⌘", false))
                .child(query_keycap("J", false)),
        )
        .into_any_element()
}

pub(super) fn query_keycap(text: &'static str, primary: bool) -> impl IntoElement {
    div()
        .h(ui_px(16.))
        .min_w(ui_px(16.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(ui_px(3.))
        .px(ui_px(4.))
        .bg(if primary {
            gpui::rgba(0x0000002e)
        } else {
            PANEL_RAISED.rgba()
        })
        .font_family(cellar_desktop_gpui::theme::mono_font())
        .text_size(ui_px(10.5))
        .opacity(0.7)
        .child(text)
}

pub(super) fn first_line(sql: &str) -> String {
    let line = sql
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("—")
        .trim();
    if line.chars().count() > 80 {
        format!("{}…", line.chars().take(79).collect::<String>())
    } else {
        line.to_owned()
    }
}
