use cellar_core::driver::Engine;
use gpui::{div, prelude::*, AnyElement};
use gpui_component::Icon;

use cellar_desktop_gpui::theme::{
    ui_px, BORDER, FG_DISABLED, FG_MUTED, FG_SECONDARY, INSET, PANEL_RAISED,
};

pub(super) fn dialect_label(engine: Engine) -> &'static str {
    match engine {
        Engine::Postgres => "PostgreSQL",
        Engine::MySql => "MySQL",
        Engine::Sqlite => "SQLite",
        Engine::Mssql => "SQL Server",
        Engine::Azure => "Azure SQL",
        Engine::Firestore => "Firestore",
        Engine::Convex => "Convex",
        Engine::Cosmos => "Cosmos DB",
        Engine::Supabase => "Supabase",
        Engine::Neon => "Neon",
        Engine::PlanetScale => "PlanetScale",
    }
}

pub(super) fn engine_color(engine: Engine) -> gpui::Rgba {
    gpui::rgb(match engine {
        Engine::Postgres => 0x4f8ff7,
        Engine::MySql => 0xf6a44a,
        Engine::Mssql => 0xd97a5a,
        Engine::Azure => 0x5bb8e0,
        Engine::Sqlite => 0xa78bfa,
        Engine::Firestore => 0xf4c542,
        Engine::Convex => 0xf25c4d,
        Engine::Cosmos => 0x6b5ce7,
        Engine::Supabase => 0x3ecf8e,
        Engine::Neon => 0x00e599,
        Engine::PlanetScale => 0xc8ccd4,
    })
}

pub(super) fn short_driver_version(version: &str) -> String {
    let mut words = version.split_whitespace();
    match (words.next(), words.next()) {
        (Some(product), Some(number)) if number.starts_with(|ch: char| ch.is_ascii_digit()) => {
            format!("{product} {number}")
        }
        _ => version.chars().take(30).collect(),
    }
}

pub(super) fn title_crumb(path: &'static str, label: String, icon_size: f32) -> gpui::Div {
    div()
        .min_w_0()
        .max_w(ui_px(130.))
        .flex()
        .items_center()
        .gap(ui_px(5.))
        .px(ui_px(6.))
        .text_size(ui_px(14.))
        .text_color(FG_SECONDARY)
        .child(
            Icon::empty()
                .path(path)
                .size(ui_px(icon_size))
                .flex_shrink_0(),
        )
        .child(div().truncate().child(label))
}

pub(super) fn title_database_crumb(label: String, engine: Engine) -> gpui::Div {
    div()
        .min_w_0()
        .max_w(ui_px(130.))
        .flex()
        .items_center()
        .gap(ui_px(5.))
        .px(ui_px(6.))
        .text_size(ui_px(14.))
        .text_color(FG_SECONDARY)
        .child(
            div()
                .size(ui_px(6.))
                .flex_shrink_0()
                .rounded(ui_px(3.))
                .bg(engine_color(engine)),
        )
        .child(div().truncate().child(label))
}

pub(super) fn title_separator() -> impl IntoElement {
    Icon::empty()
        .path("icons/chevron-right.svg")
        .size(ui_px(11.))
        .text_color(FG_DISABLED)
        .flex_shrink_0()
}

pub(super) fn keycap(text: &'static str) -> impl IntoElement {
    div()
        .min_w(ui_px(16.))
        .h(ui_px(16.))
        .px(ui_px(4.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(ui_px(3.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL_RAISED)
        .font_family(cellar_desktop_gpui::theme::mono_font())
        .text_size(ui_px(11.))
        .text_color(FG_MUTED)
        .child(text)
}

pub(super) fn bottom_empty(
    title: impl Into<gpui::SharedString>,
    detail: impl Into<gpui::SharedString>,
) -> AnyElement {
    let title = title.into();
    let detail = detail.into();
    div()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .bg(INSET)
        .text_color(FG_MUTED)
        .child(
            div()
                .text_size(ui_px(13.))
                .text_color(FG_SECONDARY)
                .child(title),
        )
        .when(!detail.is_empty(), |element| {
            element.child(
                div()
                    .max_w(ui_px(360.))
                    .text_center()
                    .text_size(ui_px(11.5))
                    .child(detail),
            )
        })
        .into_any_element()
}

pub(super) fn disabled_icon(path: &'static str, size: f32) -> impl IntoElement {
    div()
        .size(ui_px(24.))
        .flex()
        .items_center()
        .justify_center()
        .text_color(FG_MUTED)
        .opacity(0.45)
        .child(Icon::empty().path(path).size(ui_px(size)))
}
