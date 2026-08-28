use gpui::{div, prelude::*, px, AnyElement, Context};
use gpui_component::{scroll::ScrollableElement, Icon};

use super::{
    settings_data::static_segment,
    settings_workspace::{content, row, section, section_separator, section_with_sub, toggle},
    CellarApp,
};
use cellar_desktop_gpui::theme::{
    accent, accent_soft, ACCENT, BORDER, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL_MUTED,
};

impl CellarApp {
    pub(super) fn privacy_settings(&self) -> AnyElement {
        let count = self.model.connections().len();
        content()
            .child(section(
                "Telemetry",
                vec![
                    row(
                        "Send anonymous usage stats",
                        Some("counts of feature use, no query content"),
                        toggle("privacy-usage", false, false).into_any_element(),
                    ),
                    row(
                        "Send crash reports",
                        Some("stack traces only, never DB contents"),
                        toggle("privacy-crashes", false, false).into_any_element(),
                    ),
                ],
            ))
            .child(section_separator())
            .child(section_with_sub(
                "Stored locally only",
                Some("Cellar never uploads any of these. Open ~/.cellar to inspect."),
                vec![local_storage_row(count)],
            ))
            .into_any_element()
    }

    pub(super) fn updates_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let notes = match &self.updater_status {
            super::updater::UpdateStatus::Available(update) => update.notes.clone(),
            _ => None,
        };
        content()
            .child(section(
                "Updates",
                vec![
                    update_status(self, cx),
                    row(
                        "Channel",
                        None,
                        static_segment("update-channel", &["stable", "beta", "nightly"], 0),
                    ),
                    row(
                        "Auto-install on quit",
                        None,
                        toggle("update-auto-install", false, false).into_any_element(),
                    ),
                ],
            ))
            .child(section_separator())
            .child(section_with_sub(
                "What's new",
                Some(concat!("Recent changes in v", env!("CARGO_PKG_VERSION"))),
                vec![changelog(notes)],
            ))
            .into_any_element()
    }

    pub(super) fn about_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        content()
            .child(section(
                "About",
                vec![div()
                    .flex()
                    .items_start()
                    .gap_4()
                    .child(
                        Icon::empty()
                            .path("icons/cellar-mark.svg")
                            .size(px(48.))
                            .text_color(ACCENT),
                    )
                    .child(
                        div()
                            .child(
                                div()
                                    .text_size(px(19.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Cellar"),
                            )
                            .child(
                                div()
                                    .mb_2()
                                    .text_color(FG_SECONDARY)
                                    .child("A fast, native database client with AI built in."),
                            )
                            .child(
                                div()
                                    .mb_2()
                                    .flex()
                                    .gap_1()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_size(px(11.5))
                                    .text_color(FG_SECONDARY)
                                    .child(concat!(
                                        "v",
                                        env!("CARGO_PKG_VERSION"),
                                        " · development build"
                                    ))
                                    .child(div().text_color(FG_MUTED).child("·"))
                                    .child("MIT licensed")
                                    .child(div().text_color(FG_MUTED).child("·"))
                                    .child("commit unavailable"),
                            )
                            .child(
                                div()
                                    .mb_2()
                                    .flex()
                                    .gap_1()
                                    .text_color(FG_SECONDARY)
                                    .child("built by")
                                    .child(link(
                                        "about-matt",
                                        "Matt List",
                                        "https://x.com/codermatt",
                                        cx,
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .text_size(px(12.))
                                    .child(disabled_link("docs"))
                                    .child(div().text_color(FG_MUTED).child("·"))
                                    .child(link(
                                        "about-github",
                                        "github",
                                        "https://github.com/MRL-00/cellar",
                                        cx,
                                    ))
                                    .child(div().text_color(FG_MUTED).child("·"))
                                    .child(link(
                                        "about-changelog",
                                        "changelog",
                                        "https://github.com/MRL-00/cellar/releases",
                                        cx,
                                    ))
                                    .child(div().text_color(FG_MUTED).child("·"))
                                    .child(disabled_link("acknowledgements")),
                            ),
                    )
                    .into_any_element()],
            ))
            .into_any_element()
    }
}

fn local_storage_row(count: usize) -> AnyElement {
    div()
        .w_full()
        .h(px(34.))
        .flex()
        .items_center()
        .gap_2()
        .rounded(px(5.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL_MUTED)
        .px_2()
        .child(
            div()
                .w(px(160.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .child("Connections"),
        )
        .child(div().flex_1().text_color(FG_SECONDARY).child(format!(
            "{count} {}",
            if count == 1 {
                "connection"
            } else {
                "connections"
            }
        )))
        .child(
            div()
                .font_family(cellar_desktop_gpui::theme::mono_font())
                .text_size(px(11.5))
                .text_color(FG_MUTED)
                .child("~/.cellar/connections.toml"),
        )
        .child(
            Icon::empty()
                .path("icons/chevron-right.svg")
                .size(px(10.))
                .text_color(FG_MUTED),
        )
        .into_any_element()
}

fn update_status(app: &CellarApp, cx: &mut Context<CellarApp>) -> AnyElement {
    let status = app.updater_status.label();
    let can_check = app.updater_status.can_check();
    let can_install = app.updater_status.can_install();
    let last_checked = app
        .updater_last_checked
        .as_deref()
        .map(|value| format!("last checked {value}"))
        .unwrap_or_else(|| "last checked never".into());
    div()
        .mb_2()
        .flex()
        .items_center()
        .justify_between()
        .rounded(px(5.))
        .border_1()
        .border_color(BORDER)
        .bg(INSET)
        .px_3()
        .py_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .child(concat!("v", env!("CARGO_PKG_VERSION"))),
                )
                .child(
                    Icon::empty()
                        .path("icons/info.svg")
                        .size(px(11.))
                        .text_color(FG_SECONDARY),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(FG_SECONDARY)
                        .child(status),
                )
                .child(div().text_color(FG_MUTED).child(last_checked)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .when(can_install, |actions| {
                    actions.child(
                        update_button(
                            "update-install",
                            "icons/download.svg",
                            "Download & install",
                            true,
                        )
                        .on_click(
                            cx.listener(|this, _, _, cx| this.download_and_install_update(cx)),
                        ),
                    )
                })
                .child(
                    update_button("update-check", "icons/power.svg", "Check now", false)
                        .when(!can_check, |button| button.cursor_default().opacity(0.7))
                        .when(can_check, |button| {
                            button
                                .on_click(cx.listener(|this, _, _, cx| this.check_for_updates(cx)))
                        }),
                ),
        )
        .into_any_element()
}

fn update_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    primary: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .h(px(26.))
        .flex()
        .items_center()
        .gap_1()
        .rounded(px(4.))
        .border_1()
        .border_color(if primary { accent(0.32) } else { BORDER.rgba() })
        .bg(if primary {
            accent_soft()
        } else {
            PANEL_MUTED.rgba()
        })
        .px_2()
        .text_size(px(12.))
        .text_color(if primary { ACCENT } else { FG_SECONDARY })
        .cursor_pointer()
        .hover(|style| style.text_color(FG))
        .child(Icon::empty().path(icon).size(px(11.)))
        .child(label)
}

fn changelog(update_notes: Option<String>) -> AnyElement {
    let source = update_notes.unwrap_or_else(|| include_str!("../../../../CHANGELOG.md").into());
    div()
        .max_h(px(260.))
        .overflow_y_scrollbar()
        .rounded(px(5.))
        .border_1()
        .border_color(BORDER)
        .bg(INSET)
        .px_3()
        .py_2()
        .text_size(px(12.5))
        .line_height(px(19.375))
        .text_color(FG_SECONDARY)
        .children(source.lines().map(changelog_line))
        .into_any_element()
}

fn changelog_line(line: &str) -> AnyElement {
    let line = line.trim_end();
    if line.trim().is_empty() {
        return div().h(px(6.)).into_any_element();
    }
    if let Some(text) = line.trim_start_matches('#').strip_prefix(' ') {
        return changelog_inline(text)
            .mt_2()
            .mb_1()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(FG)
            .into_any_element();
    }
    if let Some(text) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
        return div()
            .flex()
            .gap(px(6.))
            .pl_1()
            .child(div().text_color(FG_MUTED).child("•"))
            .child(changelog_inline(text))
            .into_any_element();
    }
    changelog_inline(line).into_any_element()
}

fn changelog_inline(text: &str) -> gpui::Div {
    div().flex().flex_wrap().children(
        text.split("**")
            .enumerate()
            .filter(|(_, part)| !part.is_empty())
            .map(|(index, part)| {
                div()
                    .when(index % 2 == 1, |element| {
                        element
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(FG)
                    })
                    .child(part.to_owned())
            }),
    )
}

fn link(
    id: &'static str,
    label: &'static str,
    href: &'static str,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .text_decoration_solid()
        .text_color(FG_SECONDARY)
        .hover(|style| style.text_color(FG))
        .child(label)
        .on_click(cx.listener(move |_, _, _, cx| cx.open_url(href)))
        .into_any_element()
}

fn disabled_link(label: &'static str) -> AnyElement {
    div()
        .text_decoration_solid()
        .text_color(FG_MUTED)
        .opacity(0.7)
        .child(label)
        .into_any_element()
}
