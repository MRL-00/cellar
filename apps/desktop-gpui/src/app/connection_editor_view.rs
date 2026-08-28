use cellar_core::driver::{Engine, EnvTag, SslMode};
use gpui::{div, prelude::*, px, AnyElement, Context, Entity, SharedString};
use gpui_component::{input::InputState, scroll::ScrollableElement, Icon};

use super::{
    connection_editor::{ConnectionEditor, ConnectionTab, EditorBusy, ENGINES},
    connection_editor_support::{engine_color, optional_text, text},
    CellarApp,
};
use cellar_desktop_gpui::theme::{
    accent, ACCENT, ACCENT_FG, BORDER, BORDER_DIVIDER, BORDER_STRONG, FG, FG_MUTED, FG_SECONDARY,
    FG_TERTIARY, INSET, PANEL, PANEL_MUTED, PANEL_RAISED, WARN,
};
use cellar_desktop_gpui::widgets::compact_input;

const SWATCHES: [(&str, u32); 7] = [
    ("#4f8ff7", 0x4f8ff7),
    ("#f6a44a", 0xf6a44a),
    ("#d97a5a", 0xd97a5a),
    ("#5bb8e0", 0x5bb8e0),
    ("#a78bfa", 0xa78bfa),
    ("#4ade80", 0x4ade80),
    ("#f87171", 0xf87171),
];

impl CellarApp {
    pub(super) fn connection_editor_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let editor = self
            .connection_editor
            .as_ref()
            .expect("connection editor overlay requires state");
        let existing = editor.original.is_some();
        let can_save = can_save(editor, cx);

        div()
            .id("connection-editor-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(gpui::relative(0.08))
            .bg(cellar_desktop_gpui::theme::overlay())
            .on_click(cx.listener(|this, _, _, cx| {
                this.connection_editor = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("connection-editor-modal")
                    .w(px(760.))
                    .max_h(gpui::relative(0.84))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(PANEL)
                    .shadow_lg()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(editor_header(existing, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scrollbar()
                            .px_4()
                            .pt_3()
                            .pb_4()
                            .child(engine_picker(editor, cx))
                            .child(tab_bar(editor, cx))
                            .child(match editor.tab {
                                ConnectionTab::General => general_panel(editor, cx),
                                ConnectionTab::Ssh => ssh_panel(editor, cx),
                                ConnectionTab::Ssl => ssl_panel(editor, cx),
                                ConnectionTab::Options => options_panel(editor),
                            }),
                    )
                    .child(editor_footer(editor, existing, can_save, cx)),
            )
            .into_any_element()
    }
}

fn editor_header(existing: bool, cx: &mut Context<CellarApp>) -> AnyElement {
    div()
        .h(px(38.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(BORDER)
        .pl(px(14.))
        .pr_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    Icon::empty()
                        .path("icons/database.svg")
                        .size(px(14.))
                        .text_color(ACCENT),
                )
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(FG)
                        .child(if existing {
                            "Edit connection"
                        } else {
                            "New connection"
                        }),
                ),
        )
        .child(
            div()
                .id("close-connection-editor")
                .tab_index(0)
                .cursor_pointer()
                .size(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(4.))
                .hover(|style| style.bg(PANEL_RAISED))
                .child(
                    Icon::empty()
                        .path("icons/close.svg")
                        .size(px(13.))
                        .text_color(FG_MUTED),
                )
                .on_click(cx.listener(|this, _, _, cx| {
                    this.connection_editor = None;
                    cx.notify();
                })),
        )
        .into_any_element()
}

fn engine_picker(editor: &ConnectionEditor, cx: &mut Context<CellarApp>) -> AnyElement {
    div()
        .mb(px(14.))
        .flex()
        .flex_wrap()
        .gap(px(6.))
        .children(ENGINES.map(|engine| {
            let active = editor.engine == engine;
            let rgb = engine_rgb(engine);
            div()
                .id(SharedString::from(format!(
                    "connection-engine:{}",
                    engine.as_str()
                )))
                .tab_index(0)
                .cursor_pointer()
                .w(px(98.8))
                .h(px(66.))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(6.))
                .rounded(px(6.))
                .border_1()
                .border_color(if active { ACCENT } else { BORDER })
                .bg(PANEL_MUTED)
                .hover(|style| style.border_color(BORDER_STRONG))
                .child(
                    div()
                        .size(px(28.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(6.))
                        .border_1()
                        .border_color(gpui::rgba((rgb << 8) | 0x3d))
                        .bg(gpui::rgba((rgb << 8) | 0x14))
                        .child(
                            Icon::empty()
                                .path(SharedString::from(format!(
                                    "engines/{}.svg",
                                    engine.as_str()
                                )))
                                .size(px(17.)),
                        ),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .font_weight(if active {
                            gpui::FontWeight::MEDIUM
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .text_color(if active { FG } else { FG_SECONDARY })
                        .child(engine_label(engine)),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.select_connection_engine(engine, window, cx)
                }))
        }))
        .into_any_element()
}

fn tab_bar(editor: &ConnectionEditor, cx: &mut Context<CellarApp>) -> AnyElement {
    div()
        .mb(px(14.))
        .flex()
        .gap(px(2.))
        .border_b_1()
        .border_color(BORDER)
        .children(
            [
                (ConnectionTab::General, "icons/database.svg", "General"),
                (ConnectionTab::Ssh, "icons/ssh.svg", "SSH tunnel"),
                (ConnectionTab::Ssl, "icons/lock.svg", "SSL / TLS"),
                (ConnectionTab::Options, "icons/settings.svg", "Options"),
            ]
            .map(|(tab, icon, label)| {
                let active = editor.tab == tab;
                div()
                    .id(SharedString::from(format!("connection-tab:{label}")))
                    .tab_index(0)
                    .cursor_pointer()
                    .h(px(26.))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .px(px(10.))
                    .border_b_1()
                    .border_color(if active {
                        ACCENT.rgba()
                    } else {
                        gpui::rgba(0x00000000)
                    })
                    .text_color(if active { ACCENT } else { FG_TERTIARY })
                    .child(Icon::empty().path(icon).size(px(11.)))
                    .child(label)
                    .when(tab == ConnectionTab::Ssh && editor.ssh, |element| {
                        element.child(div().size(px(5.)).rounded_full().bg(ACCENT))
                    })
                    .when(
                        tab == ConnectionTab::Ssl && editor.ssl_mode != SslMode::Disable,
                        |element| element.child(div().size(px(5.)).rounded_full().bg(ACCENT)),
                    )
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.select_connection_tab(tab, cx)),
                    )
            }),
        )
        .into_any_element()
}

fn general_panel(editor: &ConnectionEditor, cx: &mut Context<CellarApp>) -> AnyElement {
    let sqlite = editor.engine == Engine::Sqlite;
    let firestore = editor.engine == Engine::Firestore;
    let convex = editor.engine == Engine::Convex;
    let cosmos = editor.engine == Engine::Cosmos;
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(form_row(
            "Name",
            Some("Shown in the sidebar"),
            input_box(&editor.name, false, None),
        ))
        .when(!sqlite, |element| {
            element.child(form_row(
                host_label(editor.engine),
                None,
                div()
                    .flex()
                    .min_w_0()
                    .items_center()
                    .gap(px(6.))
                    .child(input_box(&editor.host, true, None))
                    .child(div().text_color(FG_MUTED).child(":"))
                    .child(input_box(&editor.port, true, Some(70.)))
                    .into_any_element(),
            ))
        })
        .when(!convex, |element| {
            element.child(form_row(
                database_label(editor.engine),
                cosmos.then_some("Optional — leave blank to list every database"),
                input_box(&editor.database, true, None),
            ))
        })
        .when(!sqlite && !convex && !cosmos, |element| {
            element.child(form_row(
                if firestore { "Database ID" } else { "User" },
                None,
                input_box(&editor.user, true, None),
            ))
        })
        .when(!sqlite, |element| {
            element.child(form_row(
                password_label(editor.engine),
                Some(password_hint(editor)),
                input_box(&editor.password, true, None),
            ))
        })
        .child(div().my_1().h(px(1.)).bg(BORDER_DIVIDER))
        .child(form_row(
            "Accent",
            Some("Visual marker — protects against running on prod by mistake"),
            swatches(editor, cx),
        ))
        .child(form_row("Environment", None, environment(editor, cx)))
        .into_any_element()
}

fn ssh_panel(editor: &ConnectionEditor, cx: &mut Context<CellarApp>) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(form_row(
            "Use SSH tunnel",
            None,
            toggle("connection-ssh-toggle", editor.ssh)
                .on_click(cx.listener(|this, _, _, cx| this.toggle_connection_ssh(cx)))
                .into_any_element(),
        ))
        .child(
            div()
                .text_size(px(12.))
                .text_color(FG_MUTED)
                .child("SSH tunneling lands in a follow-up slice. Connect directly for now."),
        )
        .into_any_element()
}

fn ssl_panel(editor: &ConnectionEditor, cx: &mut Context<CellarApp>) -> AnyElement {
    let enabled = editor.ssl_mode != SslMode::Disable;
    div()
        .flex()
        .flex_col()
        .gap(px(10.))
        .child(form_row(
            "Use SSL / TLS",
            None,
            toggle("connection-ssl-toggle", enabled)
                .on_click(cx.listener(|this, _, _, cx| this.toggle_connection_ssl(cx)))
                .into_any_element(),
        ))
        .when(enabled, |element| {
            element.child(form_row(
                "SSL mode",
                None,
                div()
                    .flex()
                    .gap(px(1.))
                    .rounded(px(4.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(INSET)
                    .p(px(2.))
                    .children(
                        [
                            SslMode::Disable,
                            SslMode::Prefer,
                            SslMode::Require,
                            SslMode::VerifyCa,
                            SslMode::VerifyFull,
                        ]
                        .map(|mode| {
                            segment(ssl_mode_label(mode), editor.ssl_mode == mode).on_click(
                                cx.listener(move |this, _, _, cx| {
                                    this.select_connection_ssl(mode, cx)
                                }),
                            )
                        }),
                    )
                    .into_any_element(),
            ))
        })
        .into_any_element()
}

fn options_panel(editor: &ConnectionEditor) -> AnyElement {
    form_row(
        "Application name",
        None,
        input_box(&editor.application_name, true, None),
    )
}

fn editor_footer(
    editor: &ConnectionEditor,
    existing: bool,
    can_save: bool,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    div()
        .h(px(44.))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .border_t_1()
        .border_color(BORDER)
        .bg(PANEL_MUTED)
        .px_3()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    action_button(
                        "test-connection",
                        "icons/power.svg",
                        if editor.busy == Some(EditorBusy::Testing) {
                            "Testing…"
                        } else {
                            "Test connection"
                        },
                        false,
                        editor.busy.is_none(),
                    )
                    .when(editor.busy.is_none(), |button| {
                        button
                            .on_click(cx.listener(|this, _, _, cx| this.test_edited_connection(cx)))
                    }),
                )
                .child(test_pill(editor)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    action_button("cancel-connection-editor", "", "Cancel", false, true).on_click(
                        cx.listener(|this, _, _, cx| {
                            this.connection_editor = None;
                            cx.notify();
                        }),
                    ),
                )
                .child(
                    action_button(
                        "save-connection",
                        "icons/plus.svg",
                        if editor.busy == Some(EditorBusy::Saving) {
                            "Saving…"
                        } else if existing {
                            "Save changes"
                        } else {
                            "Save"
                        },
                        true,
                        can_save && editor.busy.is_none(),
                    )
                    .when(can_save && editor.busy.is_none(), |button| {
                        button
                            .on_click(cx.listener(|this, _, _, cx| this.save_edited_connection(cx)))
                    }),
                ),
        )
        .into_any_element()
}

fn test_pill(editor: &ConnectionEditor) -> AnyElement {
    if editor.busy == Some(EditorBusy::Testing) {
        return div()
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(12.))
            .text_color(FG_MUTED)
            .child(div().size(px(6.)).rounded_full().bg(ACCENT))
            .child("contacting…")
            .into_any_element();
    }
    match editor.message.as_ref() {
        Some(Ok(value)) => {
            let (duration, version) = value.split_once(" · ").unwrap_or((value, ""));
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .text_size(px(12.))
                .child(
                    div()
                        .h(px(15.))
                        .flex()
                        .items_center()
                        .gap_1()
                        .rounded(px(3.))
                        .bg(cellar_desktop_gpui::theme::accent_soft())
                        .px(px(6.))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(ACCENT)
                        .child(Icon::empty().path("icons/check.svg").size(px(10.)))
                        .child("Connection successful"),
                )
                .child(
                    div()
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .text_color(FG_SECONDARY)
                        .child(duration.to_owned()),
                )
                .child(div().text_color(FG_MUTED).child("·"))
                .when(!version.is_empty(), |element| {
                    element.child(
                        div()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .text_color(FG_SECONDARY)
                            .child(version.to_owned()),
                    )
                })
                .into_any_element()
        }
        Some(Err(error)) => div()
            .max_w(px(420.))
            .flex()
            .items_center()
            .gap_1()
            .truncate()
            .text_color(WARN)
            .child(Icon::empty().path("icons/triangle-alert.svg").size(px(10.)))
            .child(error.clone())
            .into_any_element(),
        None => div().into_any_element(),
    }
}

fn form_row(label: &'static str, hint: Option<&'static str>, content: AnyElement) -> AnyElement {
    div()
        .min_h(px(24.))
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(110.))
                .flex()
                .flex_col()
                .gap(px(1.))
                .pt(px(2.))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(FG_SECONDARY)
                .child(label)
                .when_some(hint, |element, hint| {
                    element.child(
                        div()
                            .text_size(px(11.))
                            .font_weight(gpui::FontWeight::NORMAL)
                            .text_color(FG_MUTED)
                            .child(hint),
                    )
                }),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(content),
        )
        .into_any_element()
}

fn input_box(state: &Entity<InputState>, mono: bool, width: Option<f32>) -> AnyElement {
    div()
        .h(px(26.))
        .min_w_0()
        .when_some(width, |element, width| element.w(px(width)).flex_none())
        .when(width.is_none(), |element| element.flex_1())
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(INSET)
        .when(mono, |element| {
            element.font_family(cellar_desktop_gpui::theme::mono_font())
        })
        .child(compact_input(state))
        .into_any_element()
}

fn swatches(editor: &ConnectionEditor, cx: &mut Context<CellarApp>) -> AnyElement {
    let current = optional_text(&editor.color, cx).unwrap_or_default();
    div()
        .flex()
        .gap_1()
        .children(SWATCHES.map(|(color, rgb)| {
            div()
                .id(SharedString::from(format!("connection-color:{color}")))
                .tab_index(0)
                .cursor_pointer()
                .size(px(18.))
                .rounded(px(4.))
                .border_1()
                .border_color(if current == color { FG } else { BORDER })
                .bg(gpui::rgb(rgb))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.select_connection_color(color, window, cx)
                }))
        }))
        .into_any_element()
}

fn environment(editor: &ConnectionEditor, cx: &mut Context<CellarApp>) -> AnyElement {
    div()
        .flex()
        .items_center()
        .child(
            div()
                .flex()
                .gap(px(1.))
                .rounded(px(4.))
                .border_1()
                .border_color(BORDER)
                .bg(INSET)
                .p(px(2.))
                .children([
                    EnvTag::Prod,
                    EnvTag::Staging,
                    EnvTag::Dev,
                    EnvTag::Local,
                ].map(|env| {
                    segment(env_label(env), editor.env_tag == env).on_click(cx.listener(
                        move |this, _, _, cx| this.select_connection_env(env, cx),
                    ))
                })),
        )
        .when(editor.env_tag == EnvTag::Prod, |element| {
            element.child(
                div()
                    .ml_2()
                    .flex()
                    .items_center()
                    .gap_1()
                    .text_size(px(11.5))
                    .text_color(WARN)
                    .child(Icon::empty().path("icons/triangle-alert.svg").size(px(10.)))
                    .child("prod will ask you to confirm before changing data (insert / update / delete)"),
            )
        })
        .into_any_element()
}

fn toggle(id: &'static str, on: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .relative()
        .w(px(28.))
        .h(px(16.))
        .rounded(px(10.))
        .bg(if on { ACCENT } else { PANEL_RAISED })
        .child(
            div()
                .absolute()
                .top(px(2.))
                .left(px(if on { 14. } else { 2. }))
                .size(px(12.))
                .rounded_full()
                .bg(gpui::white()),
        )
}

fn segment(label: &'static str, active: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(format!("connection-segment:{label}")))
        .tab_index(0)
        .cursor_pointer()
        .h(px(20.))
        .flex()
        .items_center()
        .rounded(px(3.))
        .px(px(10.))
        .bg(if active { PANEL_RAISED } else { INSET })
        .font_weight(if active {
            gpui::FontWeight::MEDIUM
        } else {
            gpui::FontWeight::NORMAL
        })
        .text_size(px(12.))
        .text_color(if active { FG } else { FG_TERTIARY })
        .child(label)
}

fn action_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    primary: bool,
    enabled: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(26.))
        .flex()
        .items_center()
        .gap(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(if primary { ACCENT } else { BORDER })
        .bg(if primary { ACCENT.rgba() } else { accent(0.) })
        .px(px(10.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(if primary { ACCENT_FG } else { FG_SECONDARY })
        .opacity(if enabled { 1. } else { 0.4 })
        .when(enabled, |element| element.tab_index(0).cursor_pointer())
        .when(enabled, |element| {
            element.hover(|style| {
                if primary {
                    style.bg(cellar_desktop_gpui::theme::hover_bright(ACCENT.rgba()))
                } else {
                    style.bg(PANEL_RAISED).text_color(FG)
                }
            })
        })
        .when(!icon.is_empty(), |element| {
            element.child(Icon::empty().path(icon).size(px(11.)))
        })
        .child(label)
}

fn can_save(editor: &ConnectionEditor, cx: &Context<CellarApp>) -> bool {
    let host = !text(&editor.host, cx).is_empty();
    let database = !text(&editor.database, cx).is_empty();
    let user = !text(&editor.user, cx).is_empty();
    match editor.engine {
        Engine::Sqlite => database,
        Engine::Cosmos => {
            host && (editor.original.is_some() || !text(&editor.password, cx).is_empty())
        }
        Engine::Convex => host,
        Engine::Firestore => host && database,
        _ => host && database && user,
    }
}

fn engine_label(engine: Engine) -> &'static str {
    match engine {
        Engine::Postgres => "PostgreSQL",
        Engine::MySql => "MySQL",
        Engine::Mssql => "SQL Server",
        Engine::Azure => "Azure SQL",
        Engine::Sqlite => "SQLite",
        Engine::Firestore => "Firestore",
        Engine::Convex => "Convex",
        Engine::Cosmos => "Cosmos DB",
        Engine::Supabase => "Supabase",
        Engine::Neon => "Neon",
        Engine::PlanetScale => "PlanetScale",
    }
}

fn engine_rgb(engine: Engine) -> u32 {
    u32::from_str_radix(&engine_color(engine)[1..], 16).unwrap_or(0xa78bfa)
}

fn host_label(engine: Engine) -> &'static str {
    match engine {
        Engine::Firestore => "API host",
        Engine::Convex => "Deployment host",
        Engine::Cosmos => "Account endpoint",
        _ => "Host",
    }
}

fn database_label(engine: Engine) -> &'static str {
    match engine {
        Engine::Sqlite => "Database file",
        Engine::Firestore => "Project ID",
        Engine::Cosmos => "Database",
        _ => "Database",
    }
}

fn password_label(engine: Engine) -> &'static str {
    match engine {
        Engine::Firestore => "Credentials",
        Engine::Convex => "Deploy key",
        Engine::Cosmos => "Primary key",
        _ => "Password",
    }
}

fn password_hint(editor: &ConnectionEditor) -> &'static str {
    match (editor.original.is_some(), editor.engine) {
        (true, Engine::Firestore) => "Leave blank to keep saved JSON/token",
        (false, Engine::Firestore) => {
            "Leave blank for emulator; JSON/token is stored in OS keychain"
        }
        (true, Engine::Convex) => "Leave blank to keep the saved deploy key",
        (false, Engine::Convex) => "Leave blank for a local backend; stored in OS keychain",
        (true, Engine::Cosmos) => "Leave blank to keep the saved account key",
        (false, Engine::Cosmos) => {
            "Account primary key from the Azure portal; stored in OS keychain"
        }
        (true, _) => "Leave blank to keep the saved password",
        (false, _) => "Stored in OS keychain",
    }
}

fn ssl_mode_label(mode: SslMode) -> &'static str {
    match mode {
        SslMode::Disable => "disable",
        SslMode::Prefer => "prefer",
        SslMode::Require => "require",
        SslMode::VerifyCa => "verify-ca",
        SslMode::VerifyFull => "verify-full",
    }
}

fn env_label(env: EnvTag) -> &'static str {
    match env {
        EnvTag::Prod => "prod",
        EnvTag::Staging => "staging",
        EnvTag::Dev => "dev",
        EnvTag::Local => "local",
    }
}
