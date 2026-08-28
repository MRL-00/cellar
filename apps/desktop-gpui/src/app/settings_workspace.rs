use gpui::{
    div, point, prelude::*, px, AnyElement, BoxShadow, Context, SharedString, Window,
    WindowAppearance,
};
use gpui_component::{
    button::Button,
    menu::{DropdownMenu, PopupMenuItem},
    slider::Slider,
    Theme as ComponentTheme, ThemeMode,
};

use super::{
    preferences::{Density, Theme},
    settings::SettingsCategory,
    settings_data::static_segment,
    shell_widgets::keycap,
    CellarApp,
};
use cellar_desktop_gpui::theme::{
    ACCENT, BORDER, BORDER_DIVIDER, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL, PANEL_MUTED,
    PANEL_RAISED,
};
use cellar_desktop_gpui::widgets::compact_input;

const SANS_FONTS: &[&str] = &[
    "Geist",
    "Inter",
    "SF Pro Text",
    "Helvetica Neue",
    "Arial",
    "Roboto",
    "Segoe UI",
];
const MONO_FONTS: &[&str] = &[
    "JetBrains Mono",
    "Geist Mono",
    "SF Mono",
    "Menlo",
    "Monaco",
    "Fira Code",
    "Cascadia Code",
    "Source Code Pro",
    "Consolas",
];
const ACCENTS: &[(&str, u32)] = &[
    ("#4ade80", 0x4ade80ff),
    ("#60a5fa", 0x60a5faff),
    ("#a78bfa", 0xa78bfaff),
    ("#fbbf24", 0xfbbf24ff),
    ("#e3b341", 0xe3b341ff),
    ("#e07a5f", 0xe07a5fff),
    ("#b5b3e8", 0xb5b3e8ff),
    ("#b8d670", 0xb8d670ff),
    ("#d878a8", 0xd878a8ff),
    ("#4dd4d4", 0x4dd4d4ff),
    ("#c44a4a", 0xc44a4aff),
    ("#c9a86a", 0xc9a86aff),
    ("#a8475c", 0xa8475cff),
    ("#ffd60a", 0xffd60aff),
    ("#8a8a8a", 0x8a8a8aff),
];

impl CellarApp {
    pub(super) fn settings_content(&self, cx: &mut Context<Self>) -> AnyElement {
        let query = self.settings_search.read(cx).value().trim().to_owned();
        if !query.is_empty() {
            return self.settings_search_results(&query, cx);
        }
        match self.settings_category {
            SettingsCategory::General => general_settings(),
            SettingsCategory::Appearance => self.appearance_settings(cx),
            SettingsCategory::Editor => self.editor_settings(cx),
            SettingsCategory::Grid => self.grid_settings(cx),
            SettingsCategory::Keymap => keymap_settings(),
            SettingsCategory::Connections => self.connection_settings(),
            SettingsCategory::History => self.history_settings(),
            SettingsCategory::Backups => self.backup_settings(cx),
            SettingsCategory::Ai => self.ai_settings(cx),
            SettingsCategory::Privacy => self.privacy_settings(),
            SettingsCategory::Updates => self.updates_settings(cx),
            SettingsCategory::About => self.about_settings(cx),
        }
    }

    fn appearance_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = self.preferences.theme;
        let density = self.preferences.density;
        let interface_font = self.preferences.interface_font.clone();
        let mono_font = self.preferences.mono_font.clone();
        content()
            .child(section(
                "Theme",
                vec![
                    row(
                        "Theme",
                        None,
                        div()
                            .flex()
                            .gap(px(1.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(BORDER)
                            .bg(INSET)
                            .p(px(2.))
                            .child(theme_choice(
                                "theme-system",
                                "system",
                                theme == Theme::System,
                                Theme::System,
                                cx,
                            ))
                            .child(theme_choice(
                                "theme-dark",
                                "dark",
                                theme == Theme::Dark,
                                Theme::Dark,
                                cx,
                            ))
                            .child(theme_choice(
                                "theme-light",
                                "light",
                                theme == Theme::Light,
                                Theme::Light,
                                cx,
                            ))
                            .into_any_element(),
                    ),
                    row(
                        "Accent",
                        None,
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .children(ACCENTS.iter().map(|(hex, rgba)| {
                                let hex = *hex;
                                let active = self.preferences.accent.eq_ignore_ascii_case(hex);
                                div()
                                    .id(SharedString::from(format!("accent:{hex}")))
                                    .tab_index(0)
                                    .size(px(18.))
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(gpui::rgba(0xffffff1a))
                                    .bg(gpui::rgba(*rgba))
                                    .cursor_pointer()
                                    .when(active, |swatch| {
                                        swatch.shadow(vec![
                                            BoxShadow {
                                                color: FG.rgba().into(),
                                                offset: point(px(0.), px(0.)),
                                                blur_radius: px(0.),
                                                spread_radius: px(3.),
                                            },
                                            BoxShadow {
                                                color: PANEL.rgba().into(),
                                                offset: point(px(0.), px(0.)),
                                                blur_radius: px(0.),
                                                spread_radius: px(2.),
                                            },
                                        ])
                                    })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.preferences.accent = hex.into();
                                        this.apply_appearance(window, cx);
                                    }))
                            }))
                            .into_any_element(),
                    ),
                    row(
                        "Density",
                        None,
                        div()
                            .flex()
                            .gap(px(1.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(BORDER)
                            .bg(INSET)
                            .p(px(2.))
                            .child(density_choice(
                                "density-compact",
                                "compact",
                                density == Density::Compact,
                                Density::Compact,
                                cx,
                            ))
                            .child(density_choice(
                                "density-comfortable",
                                "comfortable",
                                density == Density::Comfortable,
                                Density::Comfortable,
                                cx,
                            ))
                            .into_any_element(),
                    ),
                ],
            ))
            .child(section_separator())
            .child(section(
                "Type",
                vec![
                    row(
                        "Interface font",
                        None,
                        font_select("interface-font", interface_font, SANS_FONTS, false, cx),
                    ),
                    row(
                        "Editor / mono font",
                        None,
                        font_select("mono-font", mono_font, MONO_FONTS, true, cx),
                    ),
                    row(
                        "Font size",
                        Some("Scales the entire interface. Default is 13.5 px."),
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .h(px(26.))
                                    .w(px(70.))
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(INSET)
                                    .px_2()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .child(compact_input(&self.font_size_input)),
                            )
                            .child(div().text_color(FG_SECONDARY).child("px"))
                            .child(
                                div()
                                    .relative()
                                    .ml_2()
                                    .w(px(220.))
                                    .pt(px(14.))
                                    .child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .left_0()
                                            .right_0()
                                            .flex()
                                            .justify_between()
                                            .text_size(px(10.5))
                                            .text_color(FG_MUTED)
                                            .child("10")
                                            .child("13.5")
                                            .child("22"),
                                    )
                                    .child(Slider::new(&self.font_size_slider).w_full()),
                            )
                            .into_any_element(),
                    ),
                ],
            ))
            .child(section_separator())
            .child(section(
                "Window",
                vec![
                    row(
                        "Show traffic lights",
                        None,
                        toggle("traffic-lights", true, false).into_any_element(),
                    ),
                    row(
                        "Native window controls",
                        None,
                        toggle("native-controls", false, false).into_any_element(),
                    ),
                ],
            ))
            .child(section_separator())
            .child(section(
                "Reset",
                vec![row(
                    "Appearance",
                    Some("Restore theme, accent, density, fonts and size to defaults."),
                    action_button("reset-appearance", "Reset to defaults")
                        .on_click(cx.listener(|this, _, window, cx| {
                            let defaults = super::preferences::Preferences::default();
                            this.preferences.theme = defaults.theme;
                            this.preferences.density = defaults.density;
                            this.preferences.accent = defaults.accent;
                            this.preferences.font_size_px = defaults.font_size_px;
                            this.preferences.interface_font = defaults.interface_font;
                            this.preferences.mono_font = defaults.mono_font;
                            let font_size = this.preferences.font_size_px;
                            this.font_size_slider
                                .update(cx, |slider, cx| slider.set_value(font_size, window, cx));
                            this.font_size_input.update(cx, |input, cx| {
                                input.set_value(format!("{font_size:.1}"), window, cx)
                            });
                            this.apply_appearance(window, cx);
                            cx.notify();
                        }))
                        .into_any_element(),
                )],
            ))
            .into_any_element()
    }

    fn editor_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let editor = self.preferences.editor.clone();
        content()
            .child(section(
                "SQL editor",
                vec![
                    row(
                        "Tab size",
                        None,
                        div()
                            .flex()
                            .gap(px(1.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(BORDER)
                            .bg(INSET)
                            .p(px(2.))
                            .children([2, 4, 8].map(|size| {
                                choice(
                                    format!("tab-size-{size}"),
                                    size.to_string(),
                                    editor.tab_size == size,
                                )
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.apply_editor_tab_size(size, window, cx);
                                    },
                                ))
                            }))
                            .into_any_element(),
                    ),
                    row(
                        "Show line numbers",
                        None,
                        toggle("line-numbers", editor.line_numbers, true)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.preferences.editor.line_numbers =
                                    !this.preferences.editor.line_numbers;
                                let value = this.preferences.editor.line_numbers;
                                for editor in this.editors.values() {
                                    editor.update(cx, |editor, cx| {
                                        editor.set_line_number(value, window, cx)
                                    });
                                }
                                cx.notify();
                            }))
                            .into_any_element(),
                    ),
                    row(
                        "Soft wrap",
                        Some("Also toggleable per-editor with the wrap button in the toolbar."),
                        toggle("soft-wrap", editor.soft_wrap, true)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.preferences.editor.soft_wrap =
                                    !this.preferences.editor.soft_wrap;
                                let value = this.preferences.editor.soft_wrap;
                                for (id, editor) in &this.editors {
                                    editor.update(cx, |editor, cx| {
                                        editor.set_soft_wrap(value, window, cx)
                                    });
                                    this.query_wrap.insert(*id, value);
                                }
                                cx.notify();
                            }))
                            .into_any_element(),
                    ),
                    row(
                        "Bracket matching",
                        None,
                        toggle("bracket-matching", editor.bracket_matching, false)
                            .into_any_element(),
                    ),
                ],
            ))
            .into_any_element()
    }

    fn grid_settings(&self, cx: &mut Context<Self>) -> AnyElement {
        let grid = self.preferences.grid.clone();
        content()
            .child(section(
                "Data grid",
                vec![
                    row(
                        "NULL display",
                        Some("Text shown in cells where the database value is NULL."),
                        div()
                            .flex()
                            .gap(px(1.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(BORDER)
                            .bg(INSET)
                            .p(px(2.))
                            .children(["NULL", "∅", "(empty)"].map(|value| {
                                choice(format!("null-{value}"), value, grid.null_display == value)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.preferences.grid.null_display = value.into();
                                        this.apply_grid_display_preferences(cx);
                                    }))
                            }))
                            .into_any_element(),
                    ),
                    row(
                        "Stripe alternating rows",
                        None,
                        toggle("stripe-rows", grid.stripe_rows, true)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.preferences.grid.stripe_rows =
                                    !this.preferences.grid.stripe_rows;
                                this.apply_grid_display_preferences(cx);
                            }))
                            .into_any_element(),
                    ),
                    row(
                        "Remember table sort",
                        Some("Restore the last column sort when you reopen a table. Column order and widths are always remembered."),
                        toggle("remember-sort", grid.remember_table_sort, true)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.preferences.grid.remember_table_sort =
                                    !this.preferences.grid.remember_table_sort;
                                cx.notify();
                            }))
                            .into_any_element(),
                    ),
                ],
            ))
            .into_any_element()
    }

    fn apply_grid_display_preferences(&mut self, cx: &mut Context<Self>) {
        let null_display = self.preferences.grid.null_display.clone();
        let stripe_rows = self.preferences.grid.stripe_rows;
        for grid in self.grids.values() {
            grid.update(cx, |grid, cx| {
                grid.set_display_preferences(null_display.clone(), stripe_rows, cx)
            });
        }
        cx.notify();
    }

    pub(super) fn apply_appearance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let light = match self.preferences.theme {
            Theme::Light => true,
            Theme::Dark => false,
            Theme::System => matches!(
                window.appearance(),
                WindowAppearance::Light | WindowAppearance::VibrantLight
            ),
        };
        cellar_desktop_gpui::theme::set_palette(light, &self.preferences.accent);
        cellar_desktop_gpui::theme::set_density(self.preferences.density == Density::Comfortable);
        cellar_desktop_gpui::theme::set_mono_font(&self.preferences.mono_font);
        let font_size = self.preferences.font_size_px;
        cellar_desktop_gpui::theme::set_ui_scale(font_size / 13.);
        self.font_size_slider
            .update(cx, |slider, cx| slider.set_value(font_size, window, cx));
        ComponentTheme::change(
            if light {
                ThemeMode::Light
            } else {
                ThemeMode::Dark
            },
            Some(window),
            cx,
        );
        let scale = self.preferences.font_size_px / 13.;
        let theme = ComponentTheme::global_mut(cx);
        theme.font_size = px(16. * scale);
        theme.font_family = self.preferences.interface_font.clone().into();
        theme.mono_font_size = px(14. * scale);
        theme.mono_font_family = self.preferences.mono_font.clone().into();
        let editor = std::sync::Arc::make_mut(&mut theme.highlight_theme);
        editor.style.editor_background = Some(INSET.rgba().into());
        editor.style.editor_foreground = Some(FG.rgba().into());
        editor.style.editor_active_line = None;
        editor.style.editor_line_number = Some(FG_MUTED.rgba().into());
        editor.style.editor_active_line_number = Some(ACCENT.rgba().into());
        cx.notify();
    }
}

fn theme_choice(
    id: &'static str,
    label: &'static str,
    active: bool,
    theme: Theme,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    choice(id, label, active)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.preferences.theme = theme;
            this.apply_appearance(window, cx);
        }))
        .into_any_element()
}

fn density_choice(
    id: &'static str,
    label: &'static str,
    active: bool,
    density: Density,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    choice(id, label, active)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.preferences.density = density;
            cellar_desktop_gpui::theme::set_density(density == Density::Comfortable);
            cx.notify();
        }))
        .into_any_element()
}

fn general_settings() -> AnyElement {
    content()
        .child(
            div()
                .px(px(20.))
                .pt(px(16.))
                .text_color(FG_MUTED)
                .child("General settings and more config coming soon."),
        )
        .into_any_element()
}

pub(super) fn content() -> gpui::Stateful<gpui::Div> {
    div()
        .id("settings-workspace-content")
        .flex_1()
        .min_w_0()
        .h_full()
        .overflow_y_scroll()
        .pb_6()
        .pt_1()
        .bg(PANEL)
}

pub(super) fn section(title: &'static str, rows: Vec<AnyElement>) -> AnyElement {
    section_with_sub(title, None, rows)
}

pub(super) fn section_separator() -> AnyElement {
    div()
        .mt(px(6.))
        .h(px(1.))
        .bg(BORDER_DIVIDER)
        .into_any_element()
}

pub(super) fn section_with_sub(
    title: &'static str,
    sub: Option<&'static str>,
    rows: Vec<AnyElement>,
) -> AnyElement {
    div()
        .px_6()
        .pt(px(18.))
        .pb_1()
        .child(
            div()
                .mb_3()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(FG)
                        .child(title),
                )
                .when_some(sub, |element, sub| {
                    element.child(div().mt(px(1.)).text_color(FG_SECONDARY).child(sub))
                }),
        )
        .child(div().flex().flex_col().gap_2().children(rows))
        .into_any_element()
}

pub(super) fn row(
    label: &'static str,
    hint: Option<&'static str>,
    control: AnyElement,
) -> AnyElement {
    div()
        .min_h(px(24.))
        .flex()
        .items_center()
        .gap(px(14.))
        .py_1()
        .child(
            div()
                .w(px(180.))
                .flex_shrink_0()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(FG_SECONDARY)
                        .child(label),
                )
                .when_some(hint, |element, hint| {
                    element.child(
                        div()
                            .mt(px(2.))
                            .text_size(px(11.5))
                            .text_color(FG_MUTED)
                            .child(hint),
                    )
                }),
        )
        .child(div().flex_1().min_w_0().child(control))
        .into_any_element()
}

pub(super) fn choice(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    active: bool,
) -> gpui::Stateful<gpui::Div> {
    let id = id.into();
    let label = label.into();
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .h(px(20.))
        .flex()
        .items_center()
        .rounded(px(3.))
        .bg(if active { PANEL_RAISED } else { INSET })
        .px(px(10.))
        .text_size(px(12.))
        .text_color(if active { FG } else { FG_SECONDARY })
        .child(label)
}

pub(super) fn toggle(id: &'static str, on: bool, enabled: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .relative()
        .w(px(28.))
        .h(px(16.))
        .rounded(px(10.))
        .bg(if on { ACCENT } else { PANEL_RAISED })
        .opacity(if enabled { 1. } else { 0.85 })
        .when(enabled, |element| element.tab_index(0).cursor_pointer())
        .child(
            div()
                .absolute()
                .top(px(2.))
                .left(px(if on { 14. } else { 2. }))
                .size(px(12.))
                .rounded(px(6.))
                .bg(gpui::white()),
        )
}

pub(super) fn action_button(id: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .h(px(26.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL_MUTED)
        .px_2()
        .text_color(FG_SECONDARY)
        .hover(|style| style.text_color(FG))
        .child(label)
}

fn font_select(
    id: &'static str,
    value: String,
    fonts: &'static [&'static str],
    mono: bool,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    let app = cx.weak_entity();
    Button::new(id)
        .label(value.clone())
        .dropdown_caret(true)
        .h(px(26.))
        .w_full()
        .outline()
        .dropdown_menu(move |menu, _, _| {
            fonts.iter().fold(menu.min_w(px(220.)), |menu, font| {
                let font = (*font).to_owned();
                let app = app.clone();
                menu.item(
                    PopupMenuItem::new(font.clone())
                        .checked(font == value)
                        .on_click(move |_, _, cx| {
                            app.update(cx, |this, cx| {
                                if mono {
                                    this.preferences.mono_font = font.clone();
                                    cellar_desktop_gpui::theme::set_mono_font(&font);
                                } else {
                                    this.preferences.interface_font = font.clone();
                                }
                                cx.notify();
                            })
                            .ok();
                        }),
                )
            })
        })
        .into_any_element()
}

fn keymap_settings() -> AnyElement {
    const GROUPS: &[(&str, &[(&str, &[&str])])] = &[
        (
            "Workspace",
            &[
                ("Command palette", &["⌘", "K"]),
                ("New connection", &["⌘", "N"]),
                ("New SQL tab", &["⌘", "T"]),
                ("Close tab", &["⌘", "W"]),
                ("Settings", &["⌘", ","]),
                ("Toggle sidebar", &["⌘", "B"]),
                ("Toggle AI panel", &["⌘", "J"]),
                ("Toggle results panel", &["⌘", "↓"]),
            ],
        ),
        (
            "Editor",
            &[
                ("Run statement", &["⌘", "⏎"]),
                ("Run selection", &["⌥", "⏎"]),
                ("Format", &["⌥", "⇧", "F"]),
                ("Accept ghost text", &["Tab"]),
                ("Show schema for symbol", &["F12"]),
            ],
        ),
        (
            "Grid",
            &[
                ("Edit cell", &["⏎"]),
                ("Revert cell", &["Esc"]),
                ("Commit changes", &["⌘", "S"]),
                ("Revert all pending", &["⌘", "⇧", "Z"]),
                ("Set NULL", &["⌘", "⌫"]),
            ],
        ),
    ];
    GROUPS
        .iter()
        .fold(
            content().child(section_with_sub(
                "Keymap",
                Some("Pick a preset or rebind any individual shortcut."),
                vec![row(
                    "Preset",
                    None,
                    static_segment(
                        "keymap-preset",
                        &["Cellar", "DataGrip", "VS Code", "Linear"],
                        0,
                    ),
                )],
            )),
            |content, (group, shortcuts)| {
                content.child(section(
                    group,
                    vec![div()
                        .flex()
                        .flex_col()
                        .children(shortcuts.iter().map(|(label, keys)| {
                            div()
                                .h(px(34.))
                                .flex()
                                .items_center()
                                .gap_3()
                                .border_b_1()
                                .border_dashed()
                                .border_color(BORDER)
                                .child(div().flex_1().text_color(FG_SECONDARY).child(*label))
                                .child(
                                    div()
                                        .flex()
                                        .gap(px(2.))
                                        .children(keys.iter().map(|key| keycap(key))),
                                )
                                .child(
                                    div()
                                        .size(px(22.))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .opacity(0.6)
                                        .child(
                                            gpui_component::Icon::empty()
                                                .path("icons/edit.svg")
                                                .size(px(10.)),
                                        ),
                                )
                        }))
                        .into_any_element()],
                ))
            },
        )
        .into_any_element()
}
