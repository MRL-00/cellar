use gpui::{div, prelude::*, px, AnyElement, Context, SharedString};
use gpui_component::Icon;

use super::{
    settings_workspace::{
        choice, content, row, section, section_separator, section_with_sub, toggle,
    },
    CellarApp,
};
use cellar_desktop_gpui::theme::WARN;
use cellar_desktop_gpui::theme::{BORDER, FG_MUTED, FG_SECONDARY, INSET};

impl CellarApp {
    pub(super) fn connection_settings(&self) -> AnyElement {
        content()
            .child(section_with_sub(
                "Defaults for new connections",
                Some("Applied when you create a connection. Per-connection overrides win."),
                vec![
                    row("Read-only by default", None, toggle("default-readonly", true, false).into_any_element()),
                    row("Connection timeout", None, unit("connection-timeout", "10", "seconds")),
                    row("Keep-alive interval", None, unit("connection-keepalive", "30", "seconds")),
                    row("Application name", None, readonly("connection-app-name", "cellar (alice@laptop)", 260.)),
                ],
            ))
            .child(section_separator())
            .child(section_with_sub(
                "Production safety",
                Some("Cellar will require you to type the connection name before running these against any 'prod' connection."),
                vec![
                    row("Confirm DML on prod", None, forced_toggle("prod-confirm-dml")),
                    row("Confirm DROP / TRUNCATE on prod", None, forced_toggle("prod-confirm-ddl")),
                    row("Block UPDATE without WHERE", None, toggle("prod-block-update", true, false).into_any_element()),
                    row("Block DELETE without WHERE", None, toggle("prod-block-delete", true, false).into_any_element()),
                    row("Max rows affected before warn", None, unit("prod-row-warning", "100", "rows")),
                ],
            ))
            .into_any_element()
    }

    pub(super) fn history_settings(&self) -> AnyElement {
        content()
            .child(section(
                "Query history",
                vec![
                    row(
                        "Retain history for",
                        None,
                        static_segment(
                            "history-retain",
                            &["7 days", "30 days", "90 days", "forever"],
                            2,
                        ),
                    ),
                    row(
                        "Store query results",
                        None,
                        toggle("history-results", false, false).into_any_element(),
                    ),
                ],
            ))
            .into_any_element()
    }

    pub(super) fn backup_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        content()
            .child(section_with_sub(
                "Transfer setup",
                Some("Export your connections, appearance, and grid layouts to a file you can share or restore on another machine. Passwords are never included."),
                vec![row(
                    "Share or move your setup",
                    Some("Import lets you review each item and skip duplicates before applying."),
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            active_icon_button("setup-export", "icons/download.svg", "Export setup…")
                                .on_click(cx.listener(|this, _, _, cx| this.open_export_setup(cx))),
                        )
                        .child(
                            active_icon_button("setup-import", "icons/upload.svg", "Import setup…")
                                .on_click(cx.listener(|this, _, window, cx| this.open_import_setup(window, cx))),
                        )
                        .into_any_element(),
                )],
            ))
            .child(section_separator())
            .child(section(
                "Backups",
                vec![
                    row("Auto-snapshot before commits", Some("pg_dump --schema-only + affected rows"), toggle("backup-auto", true, false).into_any_element()),
                    row(
                        "Snapshot location",
                        None,
                        div()
                            .flex()
                            .gap_2()
                            .child(readonly("snapshot-location", "~/.cellar/snapshots", 260.))
                            .child(
                                active_icon_button(
                                    "snapshot-browse",
                                    "icons/file-text.svg",
                                    "Browse",
                                )
                                    .opacity(0.7)
                                    .cursor_default()
                                    .hover(|style| style.text_color(FG_SECONDARY)),
                            )
                            .into_any_element(),
                    ),
                    row("Retain snapshots for", None, unit("snapshot-retain", "30", "days")),
                ],
            ))
            .child(section_separator())
            .child(section(
                "Export defaults",
                vec![
                    row("Format", None, static_segment("export-format", &["CSV", "JSON", "Parquet", "SQL INSERT"], 0)),
                    row("NULL as", None, readonly("export-null", "\\N", 120.)),
                    row("Include headers", None, toggle("export-headers", true, false).into_any_element()),
                ],
            ))
            .into_any_element()
    }
}

fn readonly(id: &'static str, value: &'static str, width: f32) -> AnyElement {
    div()
        .id(id)
        .tab_index(0)
        .h(px(26.))
        .w(px(width))
        .flex()
        .items_center()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(INSET)
        .px_2()
        .font_family(cellar_desktop_gpui::theme::mono_font())
        .text_color(FG_SECONDARY)
        .opacity(0.8)
        .child(value)
        .into_any_element()
}

fn unit(id: &'static str, value: &'static str, suffix: &'static str) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(readonly(id, value, 70.))
        .child(div().text_color(FG_SECONDARY).child(suffix))
        .into_any_element()
}

fn forced_toggle(id: &'static str) -> AnyElement {
    toggle(id, true, false).bg(WARN).into_any_element()
}

pub(super) fn static_segment(
    id: &'static str,
    values: &[&'static str],
    active: usize,
) -> AnyElement {
    div()
        .flex()
        .gap(px(1.))
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(INSET)
        .p(px(2.))
        .children(values.iter().enumerate().map(|(index, value)| {
            choice(
                SharedString::from(format!("{id}:{value}")),
                *value,
                index == active,
            )
            .cursor_default()
        }))
        .into_any_element()
}

fn active_icon_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(26.))
        .flex()
        .items_center()
        .gap_1()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .px_2()
        .text_color(FG_SECONDARY)
        .cursor_pointer()
        .hover(|style| style.text_color(cellar_desktop_gpui::theme::FG))
        .child(Icon::empty().path(icon).size(px(11.)).text_color(FG_MUTED))
        .child(label)
}
