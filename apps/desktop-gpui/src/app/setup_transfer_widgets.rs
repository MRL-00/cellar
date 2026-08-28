use gpui::{div, prelude::*, px, AnyElement, Context, Entity, SharedString, Window};
use gpui_component::{input::InputState, Icon};

use super::{
    setup_transfer::{ImportDecision, ImportSetupState, ImportSummary, SetupTransfer},
    CellarApp,
};
use cellar_desktop_gpui::theme::{
    accent, hover_bright, ACCENT, BG, BORDER, BORDER_STRONG, FG, FG_MUTED, FG_SECONDARY, INSET,
    PANEL_MUTED, PANEL_RAISED, PROD,
};
use cellar_desktop_gpui::widgets::compact_input;

pub(super) fn modal_header(
    icon: &'static str,
    title: &'static str,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    div()
        .h(px(38.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap_2()
        .border_b_1()
        .border_color(BORDER)
        .pl(px(14.))
        .pr_2()
        .child(Icon::empty().path(icon).size(px(14.)).text_color(ACCENT))
        .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(title))
        .child(div().flex_1())
        .child(
            div()
                .id(SharedString::from(format!("close-{title}")))
                .tab_index(0)
                .cursor_pointer()
                .size(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                .child(
                    Icon::empty()
                        .path("icons/close.svg")
                        .size(px(13.))
                        .text_color(FG_MUTED),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    if !this.setup_import_busy() {
                        this.setup_transfer = None;
                        cx.notify();
                    }
                })),
        )
        .into_any_element()
}

pub(super) fn modal_footer() -> gpui::Div {
    div()
        .h(px(44.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap_2()
        .border_t_1()
        .border_color(BORDER)
        .bg(PANEL_MUTED)
        .px_3()
}

pub(super) fn footer_button(
    id: &'static str,
    label: &'static str,
    primary: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .h(px(26.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .border_1()
        .border_color(if primary { ACCENT } else { BORDER })
        .bg(if primary { ACCENT.rgba() } else { accent(0.) })
        .px_2()
        .text_color(if primary { BG } else { FG_SECONDARY })
        .hover(move |style| {
            if primary {
                style.bg(hover_bright(ACCENT.rgba()))
            } else {
                style
                    .bg(PANEL_RAISED)
                    .border_color(BORDER_STRONG)
                    .text_color(FG)
            }
        })
        .child(label)
}

pub(super) fn check_box(checked: bool) -> AnyElement {
    div()
        .size(px(15.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .border_1()
        .border_color(if checked { ACCENT } else { BORDER })
        .bg(if checked { ACCENT } else { INSET })
        .when(checked, |element| {
            element.child(
                Icon::empty()
                    .path("icons/check.svg")
                    .size(px(10.))
                    .text_color(BG),
            )
        })
        .into_any_element()
}

pub(super) fn section_card<E: 'static>(
    id: &'static str,
    label: &'static str,
    detail: &str,
    count: String,
    checked: bool,
    listener: E,
) -> AnyElement
where
    E: Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .mt_1()
        .flex()
        .items_start()
        .gap_2()
        .rounded(px(5.))
        .border_1()
        .border_color(if checked { accent(0.32) } else { BORDER.rgba() })
        .bg(if checked {
            accent(0.14)
        } else {
            PANEL_RAISED.rgba()
        })
        .px_3()
        .py_2()
        .child(check_box(checked))
        .child(
            div()
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(label)
                        .child(
                            div()
                                .font_family(cellar_desktop_gpui::theme::mono_font())
                                .text_size(px(11.))
                                .text_color(FG_MUTED)
                                .child(count),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(FG_MUTED)
                        .child(detail.to_owned()),
                ),
        )
        .on_click(listener)
        .into_any_element()
}

pub(super) fn review_row<E: 'static>(
    id: impl Into<SharedString>,
    label: &str,
    detail: &str,
    action: &str,
    active: bool,
    listener: E,
) -> AnyElement
where
    E: Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    div()
        .id(id.into())
        .tab_index(0)
        .cursor_pointer()
        .mt_1()
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(5.))
        .border_1()
        .border_color(BORDER)
        .bg(if active { PANEL_RAISED } else { INSET })
        .px_2()
        .py_2()
        .child(check_box(active))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .child(
                    div()
                        .truncate()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .child(label.to_owned()),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(px(11.5))
                        .text_color(FG_MUTED)
                        .child(detail.to_owned()),
                ),
        )
        .child(
            div()
                .rounded(px(3.))
                .border_1()
                .border_color(BORDER)
                .px_2()
                .py_1()
                .text_size(px(11.))
                .text_color(if active { ACCENT } else { FG_MUTED })
                .child(action.to_owned()),
        )
        .on_click(listener)
        .into_any_element()
}

pub(super) fn review_heading(title: String) -> AnyElement {
    div()
        .mt_3()
        .mb_1()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(title)
        .into_any_element()
}

pub(super) fn bulk_button<E: 'static>(
    id: impl Into<SharedString>,
    label: &'static str,
    listener: E,
) -> AnyElement
where
    E: Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
{
    div()
        .id(id.into())
        .tab_index(0)
        .cursor_pointer()
        .h(px(22.))
        .flex()
        .items_center()
        .rounded(px(3.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL_RAISED)
        .px_2()
        .text_size(px(11.5))
        .text_color(FG_SECONDARY)
        .on_click(listener)
        .child(label)
        .into_any_element()
}

pub(super) fn import_source(
    raw: Entity<InputState>,
    error: Option<String>,
    loading: bool,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    div()
        .flex_1()
        .p_5()
        .child(
            div()
                .text_size(px(16.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child("Choose a Cellar setup file"),
        )
        .child(
            div().mt_1().text_color(FG_SECONDARY).child(
                "Review connections, settings, and grid layouts before anything is applied. Passwords are never imported.",
            ),
        )
        .child(
            div()
                .id("pick-setup-source")
                .cursor_pointer()
                .mt_4()
                .h(px(96.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(6.))
                .border_1()
                .border_color(BORDER)
                .bg(INSET)
                .text_color(if loading { FG_MUTED } else { ACCENT })
                .child(if loading {
                    "Reading setup…"
                } else {
                    "Choose .json file"
                })
                .when(!loading, |element| {
                    element.tab_index(0).on_click(cx.listener(|this, _, _, cx| this.choose_setup_file(cx)))
                }),
        )
        .child(
            div()
                .my_3()
                .flex()
                .items_center()
                .gap_2()
                .text_size(px(11.5))
                .text_color(FG_MUTED)
                .child(div().h(px(1.)).flex_1().bg(BORDER))
                .child("or paste JSON")
                .child(div().h(px(1.)).flex_1().bg(BORDER)),
        )
        .child(
            div()
                .h(px(180.))
                .rounded(px(5.))
                .border_1()
                .border_color(BORDER)
                .bg(INSET)
                .font_family(cellar_desktop_gpui::theme::mono_font())
                .child(compact_input(&raw)),
        )
        .when_some(error, |element, error| {
            element.child(div().mt_2().text_color(PROD).child(error))
        })
        .into_any_element()
}

pub(super) fn import_result(summary: ImportSummary) -> AnyElement {
    let mut lines = Vec::new();
    for (count, label) in [
        (summary.connections_added, "connection(s) added"),
        (summary.connections_replaced, "connection(s) replaced"),
        (summary.connections_skipped, "connection(s) skipped"),
        (summary.layouts_added, "layout(s) added"),
        (summary.layouts_replaced, "layout(s) replaced"),
        (summary.layouts_skipped, "layout(s) skipped"),
    ] {
        if count > 0 {
            lines.push(format!("{count} {label}"));
        }
    }
    if summary.settings_applied {
        lines.push("appearance & settings applied".into());
    }
    let imported = summary.connections_added + summary.connections_replaced;
    div()
        .flex_1()
        .p_4()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(
                    Icon::empty()
                        .path("icons/check.svg")
                        .size(px(13.))
                        .text_color(ACCENT),
                )
                .child("Setup imported"),
        )
        .child(
            div()
                .mt_3()
                .text_color(FG_SECONDARY)
                .children(
                    lines
                        .into_iter()
                        .map(|line| div().child(format!("• {line}"))),
                ),
        )
        .when(imported > 0, |element| {
            element.child(
                div()
                    .mt_3()
                    .flex()
                    .items_start()
                    .gap_2()
                    .rounded(px(4.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(INSET)
                    .px_3()
                    .py_2()
                    .text_color(FG_SECONDARY)
                    .child(
                        Icon::empty()
                            .path("icons/lock.svg")
                            .size(px(12.))
                            .text_color(FG_MUTED),
                    )
                    .child("Imported connections have no password yet — open each one and set its credentials before connecting."),
            )
        })
        .into_any_element()
}

pub(super) fn connection_hint(connection: &cellar_core::driver::ConnectionConfig) -> String {
    if connection.engine == cellar_core::driver::Engine::Sqlite {
        return connection.database.clone();
    }
    format!(
        "{}{}{}{}",
        if connection.user.is_empty() {
            String::new()
        } else {
            format!("{}@", connection.user)
        },
        if connection.host.is_empty() {
            "localhost"
        } else {
            &connection.host
        },
        if connection.port == 0 {
            String::new()
        } else {
            format!(":{}", connection.port)
        },
        if connection.database.is_empty() {
            String::new()
        } else {
            format!("/{}", connection.database)
        }
    )
}

pub(super) fn decision_label(decision: ImportDecision) -> &'static str {
    match decision {
        ImportDecision::Skip => "skip",
        ImportDecision::Add => "add",
        ImportDecision::Replace => "replace",
        ImportDecision::Copy => "copy",
    }
}

pub(super) fn set_export_message(
    this: &gpui::WeakEntity<CellarApp>,
    cx: &mut gpui::AsyncApp,
    message: Result<String, String>,
) {
    this.update(cx, |this, cx| {
        if let Some(SetupTransfer::Export(export)) = this.setup_transfer.as_mut() {
            export.message = Some(message);
        }
        cx.notify();
    })
    .ok();
}
pub(super) fn set_import_error(
    this: &gpui::WeakEntity<CellarApp>,
    cx: &mut gpui::AsyncApp,
    error: String,
) {
    this.update(cx, |this, cx| {
        if let Some(SetupTransfer::Import(import)) = this.setup_transfer.as_mut() {
            import.state = ImportSetupState::Source { loading: false };
            import.error = Some(error);
        }
        cx.notify();
    })
    .ok();
}
pub(super) fn set_import_source(this: &gpui::WeakEntity<CellarApp>, cx: &mut gpui::AsyncApp) {
    this.update(cx, |this, cx| {
        if let Some(SetupTransfer::Import(import)) = this.setup_transfer.as_mut() {
            import.state = ImportSetupState::Source { loading: false };
        }
        cx.notify();
    })
    .ok();
}
