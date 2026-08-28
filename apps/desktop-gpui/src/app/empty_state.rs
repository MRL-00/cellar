use cellar_core::driver::Engine;
use gpui::{div, point, prelude::*, AnyElement, BoxShadow, Context, SharedString};
use gpui_component::Icon;

use super::shell_widgets::engine_color;
use super::{shell_widgets::keycap, CellarApp};
use cellar_desktop_gpui::theme::{
    accent, accent_soft, hover_bright, syntax_keyword, ui_px, ACCENT, ACCENT_FG, BG, BORDER,
    BORDER_DIVIDER, BORDER_STRONG, FG, FG_MUTED, FG_SECONDARY, PANEL, PANEL_MUTED,
};

const ENGINES: [(Engine, &str, bool); 7] = [
    (Engine::Postgres, "Postgres", true),
    (Engine::Firestore, "Firestore", true),
    (Engine::Convex, "Convex", true),
    (Engine::Cosmos, "Cosmos", true),
    (Engine::Mssql, "MSSQL", true),
    (Engine::MySql, "MySQL", true),
    (Engine::Sqlite, "SQLite", false),
];

impl CellarApp {
    pub(super) fn canonical_empty_state(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .relative()
            .flex_1()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .bg(BG)
            .child(glow(0.30, 0.20, accent(1.)))
            .child(glow(0.70, 0.80, syntax_keyword(1.)))
            .child(
                div()
                    .relative()
                    .w(ui_px(540.))
                    .rounded(ui_px(12.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(PANEL)
                    .shadow_md()
                    .px(ui_px(36.))
                    .pt(ui_px(36.))
                    .pb(ui_px(28.))
                    .text_center()
                    .child(
                        div().mb(ui_px(18.)).flex().justify_center().child(
                            div()
                                .shadow(vec![BoxShadow {
                                    color: accent_soft().into(),
                                    offset: point(ui_px(0.), ui_px(0.)),
                                    blur_radius: ui_px(14.),
                                    spread_radius: ui_px(0.),
                                }])
                                .child(
                                    Icon::empty()
                                        .path("icons/cellar-mark.svg")
                                        .size(ui_px(48.))
                                        .text_color(ACCENT),
                                ),
                        ),
                    )
                    .child(
                        div()
                            .mb_1()
                            .text_size(ui_px(21.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(FG)
                            .child("Welcome to Cellar"),
                    )
                    .child(
                        div()
                            .mb(ui_px(22.))
                            .text_size(ui_px(14.))
                            .text_color(FG_SECONDARY)
                            .child("Connect to Postgres, inspect schemas, run SQL, and browse table data."),
                    )
                    .child(self.empty_state_actions(cx))
                    .child(
                        div()
                            .mb_2()
                            .text_size(ui_px(11.))
                            .text_color(FG_MUTED)
                            .child("OR PICK AN ENGINE TO START"),
                    )
                    .child(
                        div()
                            .mb(ui_px(22.))
                            .flex()
                            .flex_wrap()
                            .gap(ui_px(6.))
                            .children(ENGINES.map(|(engine, label, available)| {
                                self.engine_card(engine, label, available, cx)
                            })),
                    )
                    .child(shortcuts())
                    .child(
                        div()
                            .border_t_1()
                            .border_color(BORDER_DIVIDER)
                            .pt_4()
                            .flex()
                            .justify_center()
                            .gap_1()
                            .text_size(ui_px(11.5))
                            .text_color(FG_MUTED)
                            .child("v0.1.0 · MIT licensed ·")
                            .child(disabled_footer_link("docs"))
                            .child("·")
                            .child(disabled_footer_link("github")),
                    ),
            )
            .into_any_element()
    }

    fn empty_state_actions(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .mb(ui_px(22.))
            .flex()
            .flex_col()
            .gap(ui_px(6.))
            .child(
                div()
                    .id("empty-new-connection")
                    .tab_index(0)
                    .h(ui_px(32.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .rounded(ui_px(6.))
                    .border_1()
                    .border_color(ACCENT)
                    .bg(ACCENT)
                    .text_color(ACCENT_FG)
                    .cursor_pointer()
                    .text_size(ui_px(14.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .hover(|style| style.bg(hover_bright(ACCENT.rgba())))
                    .child(Icon::empty().path("icons/plus.svg").size(ui_px(12.)))
                    .child("New connection")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_connection_editor(None, window, cx)
                    })),
            )
            .child(disabled_action(
                "empty-import",
                "icons/file-text.svg",
                "Import from DataGrip / DBeaver",
            ))
            .child(disabled_action(
                "empty-demo",
                "icons/cloud.svg",
                "Connect to demo database",
            ))
            .into_any_element()
    }

    fn engine_card(
        &self,
        engine: Engine,
        label: &'static str,
        available: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id(SharedString::from(format!(
                "empty-engine:{}",
                engine.as_str()
            )))
            .w(ui_px(112.))
            .h(ui_px(70.))
            .flex()
            .flex_col()
            .items_center()
            .gap(ui_px(6.))
            .pt(ui_px(10.))
            .pb(ui_px(8.))
            .rounded(ui_px(6.))
            .border_1()
            .border_color(BORDER)
            .bg(PANEL_MUTED)
            .text_size(ui_px(14.))
            .text_color(FG_SECONDARY)
            .when(!available, |card| card.opacity(0.45))
            .when(available, |card| {
                card.tab_index(0)
                    .cursor_pointer()
                    .hover(|style| style.border_color(BORDER_STRONG))
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_connection_editor(None, window, cx)
                    }))
            })
            .child(
                div()
                    .size(ui_px(26.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(ui_px(5.))
                    .border_1()
                    .border_color(gpui::Rgba {
                        a: 0.30,
                        ..engine_color(engine)
                    })
                    .bg(gpui::Rgba {
                        a: 0.12,
                        ..engine_color(engine)
                    })
                    .child(
                        Icon::empty()
                            .path(SharedString::from(format!(
                                "engines/{}.svg",
                                engine.as_str()
                            )))
                            .size(ui_px(16.))
                            .text_color(engine_color(engine)),
                    ),
            )
            .child(label)
            .into_any_element()
    }
}

fn glow(left: f32, top: f32, color: gpui::Rgba) -> AnyElement {
    div()
        .absolute()
        .left(gpui::relative(left))
        .top(gpui::relative(top))
        .children(
            [
                (960., 0.004),
                (840., 0.006),
                (720., 0.008),
                (600., 0.010),
                (480., 0.012),
                (360., 0.016),
                (240., 0.024),
            ]
            .map(|(size, alpha)| {
                div()
                    .absolute()
                    .left(ui_px(-size / 2.))
                    .top(ui_px(-size / 2.))
                    .size(ui_px(size))
                    .rounded(ui_px(size / 2.))
                    .bg(gpui::Rgba { a: alpha, ..color })
            }),
        )
        .into_any_element()
}

fn disabled_action(id: &'static str, icon: &'static str, label: &'static str) -> AnyElement {
    div()
        .id(id)
        .h(ui_px(32.))
        .flex()
        .items_center()
        .justify_center()
        .gap_2()
        .rounded(ui_px(6.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL_MUTED)
        .text_color(FG_SECONDARY)
        .opacity(0.55)
        .text_size(ui_px(14.))
        .child(Icon::empty().path(icon).size(ui_px(12.)))
        .child(label)
        .into_any_element()
}

fn shortcuts() -> AnyElement {
    div()
        .mb(ui_px(18.))
        .flex()
        .justify_center()
        .gap_4()
        .text_size(ui_px(11.5))
        .text_color(FG_MUTED)
        .child(shortcut("K", "command palette"))
        .child(shortcut("N", "new connection"))
        .child(shortcut(",", "settings"))
        .into_any_element()
}

fn shortcut(key: &'static str, label: &'static str) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(keycap("⌘"))
        .child(keycap(key))
        .child(label)
        .into_any_element()
}

fn disabled_footer_link(label: &'static str) -> AnyElement {
    div()
        .opacity(0.7)
        .text_decoration_solid()
        .child(label)
        .into_any_element()
}
