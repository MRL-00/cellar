use gpui::{div, prelude::*, px, AnyElement, Context, SharedString};
use gpui_component::{input::Input, Icon};

use super::CellarApp;
use cellar_desktop_gpui::theme::{
    ACCENT, ACCENT_FG, BG, BORDER, BORDER_DIVIDER, FG, FG_MUTED, FG_SECONDARY, FG_TERTIARY, INSERT,
    INSET, PANEL, PANEL_MUTED, PANEL_RAISED,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SettingsCategory {
    General,
    Appearance,
    Editor,
    Grid,
    Keymap,
    Connections,
    History,
    Backups,
    Ai,
    Privacy,
    Updates,
    About,
}

impl SettingsCategory {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Editor => "Editor",
            Self::Grid => "Data grid",
            Self::Keymap => "Keymap",
            Self::Connections => "Connections",
            Self::History => "Query history",
            Self::Backups => "Backups & exports",
            Self::Ai => "AI Assistant",
            Self::Privacy => "Privacy & telemetry",
            Self::Updates => "Updates",
            Self::About => "About",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::General => "icons/settings.svg",
            Self::Appearance => "icons/layout.svg",
            Self::Editor => "icons/edit.svg",
            Self::Grid => "icons/table.svg",
            Self::Keymap => "icons/terminal.svg",
            Self::Connections => "icons/database.svg",
            Self::History => "icons/history.svg",
            Self::Backups => "icons/cloud.svg",
            Self::Ai => "icons/sparkles.svg",
            Self::Privacy => "icons/lock.svg",
            Self::Updates => "icons/power.svg",
            Self::About => "icons/info.svg",
        }
    }
}

const GROUPS: &[(&str, &[SettingsCategory])] = &[
    (
        "Workspace",
        &[
            SettingsCategory::General,
            SettingsCategory::Appearance,
            SettingsCategory::Editor,
            SettingsCategory::Grid,
            SettingsCategory::Keymap,
        ],
    ),
    (
        "Data",
        &[
            SettingsCategory::Connections,
            SettingsCategory::History,
            SettingsCategory::Backups,
        ],
    ),
    ("Intelligence", &[SettingsCategory::Ai]),
    (
        "System",
        &[
            SettingsCategory::Privacy,
            SettingsCategory::Updates,
            SettingsCategory::About,
        ],
    ),
];

impl CellarApp {
    pub(crate) fn open_appearance_settings(&mut self, cx: &mut Context<Self>) {
        self.open_settings(SettingsCategory::Appearance, cx);
    }

    pub(crate) fn open_about_settings(&mut self, cx: &mut Context<Self>) {
        self.open_settings(SettingsCategory::About, cx);
    }

    pub(super) fn open_settings(&mut self, category: SettingsCategory, cx: &mut Context<Self>) {
        self.command_palette = None;
        self.command_palette_subscription = None;
        self.settings_category = category;
        self.settings_open = true;
        cx.notify();
    }

    pub(super) fn close_settings(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.settings_open = false;
        self.settings_search
            .update(cx, |search, cx| search.set_value("", window, cx));
        cx.notify();
    }

    pub(super) fn settings_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let category = self.settings_category;
        div()
            .id("settings-backdrop")
            .absolute()
            .inset_0()
            .bg(cellar_desktop_gpui::theme::overlay())
            .flex()
            .items_start()
            .justify_center()
            .pt(gpui::relative(0.08))
            .on_click(cx.listener(|this, _, window, cx| {
                this.close_settings(window, cx);
            }))
            .child(
                div()
                    .id("settings-modal")
                    .w(px(960.))
                    .h(px(660.))
                    .max_h(gpui::relative(0.9))
                    .flex()
                    .flex_col()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(PANEL)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .h(px(42.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_3()
                            .pl_3()
                            .pr_2()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                Icon::empty()
                                    .path("icons/settings.svg")
                                    .size(px(14.))
                                    .text_color(ACCENT),
                            )
                            .child(
                                div()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_size(px(14.))
                                    .child("Settings"),
                            )
                            .child(div().h(px(14.)).w(px(1.)).bg(BORDER_DIVIDER))
                            .child(
                                div()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .text_size(px(12.))
                                    .text_color(FG_SECONDARY)
                                    .child(category.label()),
                            )
                            .child(div().flex_1())
                            .child(
                                div()
                                    .w(px(260.))
                                    .h(px(24.))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .px(px(7.))
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(INSET)
                                    .text_color(FG_MUTED)
                                    .child(Icon::empty().path("icons/search.svg").size(px(11.)))
                                    .child(
                                        div().min_w_0().flex_1().h_full().child(
                                            Input::new(&self.settings_search)
                                                .h_full()
                                                .appearance(false),
                                        ),
                                    )
                                    .child(
                                        div()
                                            .rounded(px(3.))
                                            .border_1()
                                            .border_color(BORDER)
                                            .bg(PANEL_RAISED)
                                            .px_1()
                                            .font_family(cellar_desktop_gpui::theme::mono_font())
                                            .text_size(px(10.))
                                            .line_height(px(16.))
                                            .child("⌘F"),
                                    ),
                            )
                            .child(
                                div()
                                    .id("close-settings")
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .size(px(24.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.))
                                    .hover(|style| style.bg(PANEL_RAISED))
                                    .child(Icon::empty().path("icons/close.svg").size(px(13.)))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.close_settings(window, cx);
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .child(self.settings_nav(cx))
                            .child(self.settings_content(cx)),
                    )
                    .child(
                        div()
                            .h(px(44.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .border_t_1()
                            .border_color(BORDER)
                            .bg(PANEL_MUTED)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .text_size(px(12.))
                                    .text_color(FG_MUTED)
                                    .child(div().size(px(6.)).rounded(px(3.)).bg(INSERT))
                                    .child("Saved locally")
                                    .child(
                                        div()
                                            .font_family(cellar_desktop_gpui::theme::mono_font())
                                            .text_color(FG_SECONDARY)
                                            .child("browser storage"),
                                    )
                                    .child("·")
                                    .child(
                                        div()
                                            .text_color(FG_MUTED)
                                            .text_decoration_solid()
                                            .child("edit raw"),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .h(px(26.))
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .px_2()
                                            .rounded(px(4.))
                                            .border_1()
                                            .border_color(BORDER)
                                            .opacity(0.6)
                                            .text_color(FG_MUTED)
                                            .text_size(px(14.))
                                            .child(
                                                Icon::empty().path("icons/undo.svg").size(px(11.)),
                                            )
                                            .child("Reset section"),
                                    )
                                    .child(
                                        div()
                                            .id("done-settings")
                                            .tab_index(0)
                                            .cursor_pointer()
                                            .h(px(26.))
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .px_3()
                                            .rounded(px(4.))
                                            .bg(ACCENT)
                                            .text_color(ACCENT_FG)
                                            .text_size(px(14.))
                                            .hover(|style| {
                                                style.bg(cellar_desktop_gpui::theme::hover_bright(
                                                    ACCENT.rgba(),
                                                ))
                                            })
                                            .child(
                                                Icon::empty().path("icons/check.svg").size(px(11.)),
                                            )
                                            .child("Done")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.close_settings(window, cx);
                                            })),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn settings_nav(&self, cx: &mut Context<Self>) -> AnyElement {
        let query = self.settings_search.read(cx).value().trim().to_owned();
        let results = super::settings_search::search(&query);
        div()
            .id("settings-nav")
            .w(px(200.))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(BORDER)
            .bg(BG)
            .py_2()
            .overflow_y_scroll()
            .children(GROUPS.iter().flat_map(|(group, categories)| {
                let group_matches = !query.is_empty()
                    && group
                        .to_ascii_lowercase()
                        .contains(&query.to_ascii_lowercase());
                let visible = categories
                    .iter()
                    .copied()
                    .filter(|category| {
                        query.is_empty()
                            || group_matches
                            || category
                                .label()
                                .to_ascii_lowercase()
                                .contains(&query.to_ascii_lowercase())
                            || results.iter().any(|entry| entry.category == *category)
                    })
                    .collect::<Vec<_>>();
                let group_header = (!visible.is_empty()).then(|| {
                    div()
                        .px(px(14.))
                        .py_1()
                        .text_size(px(10.5))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(FG_MUTED)
                        .child(group.to_ascii_uppercase())
                        .into_any_element()
                });
                group_header
                    .into_iter()
                    .chain(visible.into_iter().map(|next| {
                        let active = next == self.settings_category;
                        let count = results
                            .iter()
                            .filter(|entry| entry.category == next)
                            .count();
                        div()
                            .id(SharedString::from(format!("settings-nav:{}", next.label())))
                            .tab_index(0)
                            .cursor_pointer()
                            .w_full()
                            .h(px(26.))
                            .flex()
                            .items_center()
                            .gap_2()
                            .pl(px(14.))
                            .pr(px(10.))
                            .border_l_2()
                            .border_color(if active {
                                ACCENT.rgba()
                            } else {
                                cellar_desktop_gpui::theme::accent(0.)
                            })
                            .bg(if active {
                                cellar_desktop_gpui::theme::accent_soft()
                            } else {
                                BG.rgba()
                            })
                            .font_weight(if active {
                                gpui::FontWeight::MEDIUM
                            } else {
                                gpui::FontWeight::NORMAL
                            })
                            .text_color(if active { ACCENT } else { FG_SECONDARY })
                            .when(!active, |row| {
                                row.hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                            })
                            .child(
                                div()
                                    .size(px(14.))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::empty()
                                            .path(next.icon())
                                            .size(px(12.))
                                            .text_color(if active { ACCENT } else { FG_TERTIARY }),
                                    ),
                            )
                            .child(div().flex_1().child(next.label()))
                            .when(!query.is_empty() && count > 0, |element| {
                                element.child(
                                    div()
                                        .font_family(cellar_desktop_gpui::theme::mono_font())
                                        .text_size(px(11.))
                                        .text_color(if active { ACCENT } else { FG_MUTED })
                                        .child(count.to_string()),
                                )
                            })
                            .when(
                                query.is_empty() && next == SettingsCategory::Ai,
                                |element| {
                                    element.child(
                                        div()
                                            .rounded(px(3.))
                                            .border_1()
                                            .border_color(cellar_desktop_gpui::theme::accent(0.32))
                                            .bg(if active {
                                                PANEL.rgba()
                                            } else {
                                                cellar_desktop_gpui::theme::accent_soft()
                                            })
                                            .px_1()
                                            .text_size(px(10.))
                                            .text_color(ACCENT)
                                            .child("BYO KEY"),
                                    )
                                },
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.settings_category = next;
                                cx.notify();
                            }))
                            .into_any_element()
                    }))
            }))
            .child(
                div()
                    .mt_auto()
                    .flex()
                    .items_center()
                    .gap_1()
                    .border_t_1()
                    .border_color(BORDER_DIVIDER)
                    .px(px(14.))
                    .py(px(10.))
                    .text_size(px(11.))
                    .text_color(FG_MUTED)
                    .child(concat!("v", env!("CARGO_PKG_VERSION")))
                    .child("·")
                    .child(div().opacity(0.7).text_decoration_solid().child("docs")),
            )
            .into_any_element()
    }
}
