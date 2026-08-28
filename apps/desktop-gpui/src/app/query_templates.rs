use cellar_runtime::query_templates::{self, QueryTemplate};
use gpui::{div, prelude::*, px, AnyElement, Context, SharedString, Window};
use gpui_component::{input::InputState, scroll::ScrollableElement, Icon};

use super::CellarApp;
use cellar_desktop_gpui::theme::{
    accent, ACCENT, ACCENT_FG, BORDER, BORDER_STRONG, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL,
    PANEL_MUTED, PANEL_RAISED, PROD,
};
use cellar_desktop_gpui::widgets::compact_input;

pub(super) struct SaveTemplateEditor {
    sql: String,
    name: gpui::Entity<InputState>,
    description: gpui::Entity<InputState>,
    saving: bool,
    error: Option<String>,
}

impl CellarApp {
    pub(super) fn refresh_query_templates(&mut self, cx: &mut Context<Self>) {
        let runtime = self.runtime.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(query_templates::list())
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result.map_err(|error| error.to_string()));
            if let Ok(templates) = result {
                this.update(cx, |this, cx| {
                    this.query_templates = templates;
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    pub(super) fn open_save_template(
        &mut self,
        tab_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.editors.get(&tab_id) else {
            return;
        };
        let sql = editor.read(cx).value().to_string();
        if sql.trim().is_empty() {
            return;
        }
        let default_name = self
            .model
            .tabs()
            .iter()
            .find(|tab| tab.id == tab_id)
            .map(|tab| tab.title.trim_end_matches(".sql").to_owned())
            .unwrap_or_else(|| "Query".into());
        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Recent orders by region")
                .default_value(default_name)
        });
        name.update(cx, |name, cx| name.focus(window, cx));
        self.save_template_editor = Some(SaveTemplateEditor {
            sql,
            name,
            description: cx.new(|cx| {
                InputState::new(window, cx)
                    .multi_line(true)
                    .placeholder("What this query is for…")
            }),
            saving: false,
            error: None,
        });
        cx.notify();
    }

    pub(super) fn save_query_template(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.save_template_editor else {
            return;
        };
        let template = QueryTemplate {
            name: editor.name.read(cx).value().trim().to_owned(),
            description: editor.description.read(cx).value().trim().to_owned(),
            sql: editor.sql.clone(),
        };
        if editor.saving || template.name.is_empty() || template.sql.trim().is_empty() {
            return;
        }
        editor.saving = true;
        editor.error = None;
        let runtime = self.runtime.clone();
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(query_templates::save(template))
                .await
                .map_err(|error| error.to_string())
                .and_then(|result| result.map_err(|error| error.to_string()));
            this.update(cx, |this, cx| match result {
                Ok(template) => {
                    this.query_templates
                        .retain(|existing| existing.name != template.name);
                    this.query_templates.push(template);
                    this.query_templates
                        .sort_by_key(|template| template.name.to_ascii_lowercase());
                    this.save_template_editor = None;
                    cx.notify();
                }
                Err(error) => {
                    if let Some(editor) = &mut this.save_template_editor {
                        editor.saving = false;
                        editor.error = Some(error);
                        cx.notify();
                    }
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(super) fn save_template_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let editor = self
            .save_template_editor
            .as_ref()
            .expect("save-template overlay requires state");
        let can_save = !editor.saving
            && !editor.name.read(cx).value().trim().is_empty()
            && !editor.sql.trim().is_empty();
        div()
            .id("save-template-backdrop")
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(gpui::relative(0.08))
            .bg(cellar_desktop_gpui::theme::overlay())
            .on_click(cx.listener(|this, _, _, cx| {
                this.save_template_editor = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("save-template-modal")
                    .w(px(520.))
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
                    .child(
                        div()
                            .h(px(38.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .pl(px(14.))
                            .pr_2()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Icon::empty()
                                            .path("icons/star.svg")
                                            .size(px(14.))
                                            .text_color(ACCENT),
                                    )
                                    .child(
                                        div()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("Save query template"),
                                    ),
                            )
                            .child(
                                div()
                                    .id("close-save-template")
                                    .tab_index(0)
                                    .size(px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(px(4.))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(PANEL_RAISED))
                                    .child(Icon::empty().path("icons/close.svg").size(px(13.)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.save_template_editor = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_y_scrollbar()
                            .px_4()
                            .py(px(14.))
                            .child(field_label("Name", None))
                            .child(
                                div()
                                    .mb_3()
                                    .h(px(30.))
                                    .w_full()
                                    .rounded(px(5.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(INSET)
                                    .px(px(10.))
                                    .child(compact_input(&editor.name)),
                            )
                            .child(field_label("Description", Some("(optional)")))
                            .child(
                                div()
                                    .mb_3()
                                    .h(px(64.))
                                    .w_full()
                                    .rounded(px(5.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(INSET)
                                    .px(px(10.))
                                    .child(compact_input(&editor.description)),
                            )
                            .child(field_label("SQL", None))
                            .child(
                                div()
                                    .max_h(px(128.))
                                    .overflow_y_scrollbar()
                                    .rounded(px(5.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(INSET)
                                    .px(px(10.))
                                    .py(px(6.))
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .line_height(px(21.))
                                    .text_color(FG_SECONDARY)
                                    .child(editor.sql.trim().to_owned()),
                            )
                            .when_some(editor.error.clone(), |body, error| {
                                body.child(div().mt_2().text_color(PROD).child(error))
                            }),
                    )
                    .child(
                        div()
                            .h(px(44.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .border_t_1()
                            .border_color(BORDER)
                            .bg(PANEL_MUTED)
                            .px_3()
                            .child(
                                modal_button("cancel-save-template", "Cancel", false).on_click(
                                    cx.listener(|this, _, _, cx| {
                                        this.save_template_editor = None;
                                        cx.notify();
                                    }),
                                ),
                            )
                            .child(
                                modal_button(
                                    "confirm-save-template",
                                    if editor.saving {
                                        "Saving…"
                                    } else {
                                        "Save template"
                                    },
                                    true,
                                )
                                .opacity(if can_save { 1. } else { 0.4 })
                                .when(can_save, |button| {
                                    button.cursor_pointer().on_click(
                                        cx.listener(|this, _, _, cx| this.save_query_template(cx)),
                                    )
                                }),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn field_label(label: &'static str, suffix: Option<&'static str>) -> AnyElement {
    div()
        .mb_1()
        .flex()
        .gap_1()
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(FG_SECONDARY)
        .child(label)
        .when_some(suffix, |label, suffix| {
            label.child(
                div()
                    .font_weight(gpui::FontWeight::NORMAL)
                    .text_color(FG_MUTED)
                    .child(suffix),
            )
        })
        .into_any_element()
}

fn modal_button(
    id: &'static str,
    label: impl Into<SharedString>,
    primary: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .h(px(26.))
        .flex()
        .items_center()
        .gap(px(5.))
        .rounded(px(4.))
        .border_1()
        .border_color(if primary {
            cellar_desktop_gpui::theme::accent(0.)
        } else {
            BORDER.rgba()
        })
        .bg(if primary { ACCENT.rgba() } else { accent(0.) })
        .px(px(10.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(if primary { ACCENT_FG } else { FG_SECONDARY })
        .when(!primary, |button| {
            button.hover(|style| {
                style
                    .bg(PANEL_RAISED)
                    .border_color(BORDER_STRONG)
                    .text_color(FG)
            })
        })
        .when(primary, |button| {
            button
                .hover(|style| style.bg(cellar_desktop_gpui::theme::hover_bright(ACCENT.rgba())))
                .child(Icon::empty().path("icons/star.svg").size(px(11.)))
        })
        .child(label.into())
}
