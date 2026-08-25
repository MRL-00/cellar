use std::collections::HashSet;

use cellar_core::schema::Schema;
use gpui::{div, prelude::*, px, AnyElement, Context, Entity, SharedString, Subscription, Window};
use gpui_component::{
    input::{Input, InputState},
    Icon,
};
use serde::{Deserialize, Serialize};

use super::CellarApp;
use cellar_desktop_gpui::theme::{
    ACCENT, ACCENT_FG, BG, BORDER, BORDER_STRONG, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL,
    PANEL_RAISED,
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct SchemaVisibilityPrefs {
    #[serde(default)]
    pub(super) hidden: HashSet<String>,
    #[serde(default)]
    pub(super) show_hidden: bool,
}

pub(super) struct SchemaVisibilityEditor {
    connection_id: String,
    database: String,
    filter: Entity<InputState>,
    _subscription: Subscription,
}

impl CellarApp {
    pub(super) fn schema_visibility_key(connection_id: &str, database: &str) -> String {
        format!("{connection_id}::{database}")
    }

    pub(super) fn visible_schemas<'a>(
        &self,
        connection_id: &str,
        database: &str,
        schemas: &'a [Schema],
    ) -> Vec<&'a Schema> {
        let key = Self::schema_visibility_key(connection_id, database);
        let prefs = self
            .schema_visibility
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let auto_hide_empty = schemas.iter().any(schema_has_objects) && !prefs.show_hidden;
        schemas
            .iter()
            .filter(|schema| schema_is_visible(schema, &prefs, auto_hide_empty))
            .collect()
    }

    pub(super) fn open_schema_visibility(
        &mut self,
        connection_id: String,
        database: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter schemas..."));
        let subscription = cx.observe(&filter, |_, _, cx| cx.notify());
        filter.update(cx, |filter, cx| filter.focus(window, cx));
        self.schema_visibility_editor = Some(SchemaVisibilityEditor {
            connection_id,
            database,
            filter,
            _subscription: subscription,
        });
        cx.notify();
    }

    pub(super) fn set_empty_schemas_visible(
        &mut self,
        connection_id: &str,
        database: &str,
        visible: bool,
        cx: &mut Context<Self>,
    ) {
        self.schema_visibility
            .entry(Self::schema_visibility_key(connection_id, database))
            .or_default()
            .show_hidden = visible;
        cx.notify();
    }

    pub(super) fn set_schema_hidden(
        &mut self,
        connection_id: &str,
        database: &str,
        schema: &str,
        hidden: bool,
        cx: &mut Context<Self>,
    ) {
        let prefs = self
            .schema_visibility
            .entry(Self::schema_visibility_key(connection_id, database))
            .or_default();
        if hidden {
            prefs.hidden.insert(schema.to_owned());
            prefs.show_hidden = false;
        } else {
            prefs.hidden.remove(schema);
        }
        cx.notify();
    }

    fn set_schema_selection(&mut self, visible: HashSet<String>, cx: &mut Context<Self>) {
        let Some(editor) = &self.schema_visibility_editor else {
            return;
        };
        let Some(database) = self
            .model
            .databases(&editor.connection_id)
            .iter()
            .find(|database| database.name == editor.database)
        else {
            return;
        };
        let hidden = database
            .schemas
            .iter()
            .map(|schema| schema.name.clone())
            .filter(|name| !visible.contains(name))
            .collect();
        let key = Self::schema_visibility_key(&editor.connection_id, &editor.database);
        self.schema_visibility.insert(
            key,
            SchemaVisibilityPrefs {
                hidden,
                show_hidden: true,
            },
        );
        cx.notify();
    }

    fn toggle_schema_visibility(&mut self, schema: String, cx: &mut Context<Self>) {
        let Some(editor) = &self.schema_visibility_editor else {
            return;
        };
        let key = Self::schema_visibility_key(&editor.connection_id, &editor.database);
        let prefs = self.schema_visibility.entry(key).or_default();
        prefs.show_hidden = true;
        if !prefs.hidden.remove(&schema) {
            prefs.hidden.insert(schema);
        }
        cx.notify();
    }

    pub(super) fn schema_visibility_overlay(&self, cx: &mut Context<Self>) -> AnyElement {
        let editor = self
            .schema_visibility_editor
            .as_ref()
            .expect("schema visibility overlay requires editor");
        let Some(database) = self
            .model
            .databases(&editor.connection_id)
            .iter()
            .find(|database| database.name == editor.database)
        else {
            return div().into_any_element();
        };
        let key = Self::schema_visibility_key(&editor.connection_id, &editor.database);
        let prefs = self
            .schema_visibility
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let auto_hide_empty = database.schemas.iter().any(schema_has_objects) && !prefs.show_hidden;
        let filter = editor.filter.read(cx).value().trim().to_lowercase();
        let schemas = database
            .schemas
            .iter()
            .filter(|schema| filter.is_empty() || schema.name.to_lowercase().contains(&filter))
            .collect::<Vec<_>>();
        let visible_count = database
            .schemas
            .iter()
            .filter(|schema| schema_is_visible(schema, &prefs, auto_hide_empty))
            .count();
        let all = database
            .schemas
            .iter()
            .map(|schema| schema.name.clone())
            .collect::<HashSet<_>>();
        let non_empty = database
            .schemas
            .iter()
            .filter(|schema| schema_has_objects(schema))
            .map(|schema| schema.name.clone())
            .collect::<HashSet<_>>();
        let empty = database
            .schemas
            .iter()
            .filter(|schema| !schema_has_objects(schema))
            .map(|schema| schema.name.clone())
            .collect::<HashSet<_>>();

        div()
            .id("schema-visibility-backdrop")
            .absolute()
            .inset_0()
            .bg(gpui::rgba(0x00000059))
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(|this, _, _, cx| {
                this.schema_visibility_editor = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("schema-visibility-modal")
                    .w(px(420.))
                    .max_h(gpui::relative(0.7))
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
                            .h(px(40.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                div()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_size(px(12.5))
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child("Visible schemas"),
                                    )
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(px(10.5))
                                            .text_color(FG_MUTED)
                                            .child(editor.database.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .id("close-schema-visibility")
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .size(px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(Icon::empty().path("icons/close.svg").size(px(12.)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.schema_visibility_editor = None;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .px_3()
                            .py_2()
                            .border_b_1()
                            .border_color(BORDER)
                            .child(
                                div()
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .gap(px(6.))
                                    .px_2()
                                    .rounded(px(4.))
                                    .border_1()
                                    .border_color(BORDER)
                                    .bg(INSET)
                                    .child(
                                        Icon::empty()
                                            .path("icons/search.svg")
                                            .size(px(11.))
                                            .text_color(FG_MUTED),
                                    )
                                    .child(
                                        Input::new(&editor.filter)
                                            .h_full()
                                            .flex_1()
                                            .appearance(false),
                                    ),
                            )
                            .child(
                                div()
                                    .mt_2()
                                    .flex()
                                    .flex_wrap()
                                    .gap_1()
                                    .child(schema_action("schemas-all", "All").on_click(
                                        cx.listener(move |this, _, _, cx| {
                                            this.set_schema_selection(all.clone(), cx)
                                        }),
                                    ))
                                    .child(schema_action("schemas-none", "None").on_click(
                                        cx.listener(|this, _, _, cx| {
                                            this.set_schema_selection(HashSet::new(), cx)
                                        }),
                                    ))
                                    .child(
                                        schema_action("schemas-non-empty", "Non-empty").on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.set_schema_selection(non_empty.clone(), cx)
                                            }),
                                        ),
                                    )
                                    .child(schema_action("schemas-empty", "Empty").on_click(
                                        cx.listener(move |this, _, _, cx| {
                                            this.set_schema_selection(empty.clone(), cx)
                                        }),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .id("schema-visibility-list")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .py_1()
                            .children(schemas.into_iter().map(|schema| {
                                let checked = !prefs.hidden.contains(&schema.name)
                                    && (!auto_hide_empty || schema_has_objects(schema));
                                let name = schema.name.clone();
                                div()
                                    .id(SharedString::from(format!(
                                        "schema-visible:{}",
                                        schema.name
                                    )))
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_3()
                                    .hover(|style| style.bg(PANEL_RAISED))
                                    .child(
                                        div()
                                            .size(px(14.))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(3.))
                                            .border_1()
                                            .border_color(if checked { ACCENT } else { BORDER })
                                            .bg(if checked { ACCENT } else { BG })
                                            .text_size(px(10.))
                                            .text_color(ACCENT_FG)
                                            .child(if checked { "✓" } else { "" }),
                                    )
                                    .child(
                                        Icon::empty()
                                            .path("icons/schema.svg")
                                            .size(px(12.))
                                            .text_color(FG_MUTED),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .truncate()
                                            .text_color(FG_SECONDARY)
                                            .child(schema.name.clone()),
                                    )
                                    .child(div().text_size(px(10.)).text_color(FG_MUTED).child(
                                        (schema.tables.len() + schema.views.len()).to_string(),
                                    ))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.toggle_schema_visibility(name.clone(), cx)
                                    }))
                            })),
                    )
                    .child(
                        div()
                            .h(px(36.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_3()
                            .border_t_1()
                            .border_color(BORDER)
                            .child(
                                div()
                                    .text_size(px(10.5))
                                    .text_color(FG_MUTED)
                                    .child(format!(
                                        "{visible_count}/{} visible",
                                        database.schemas.len()
                                    )),
                            )
                            .child(
                                div()
                                    .id("done-schema-visibility")
                                    .tab_index(0)
                                    .cursor_pointer()
                                    .h(px(24.))
                                    .flex()
                                    .items_center()
                                    .px(px(10.))
                                    .rounded(px(4.))
                                    .bg(ACCENT)
                                    .text_size(px(11.))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(ACCENT_FG)
                                    .child("Done")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.schema_visibility_editor = None;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn schema_action(id: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .h(px(20.))
        .flex()
        .items_center()
        .px_2()
        .rounded(px(3.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL_RAISED)
        .text_size(px(10.5))
        .text_color(FG_SECONDARY)
        .hover(|style| style.border_color(BORDER_STRONG).text_color(FG))
        .child(label)
}

fn schema_has_objects(schema: &Schema) -> bool {
    !schema.tables.is_empty() || !schema.views.is_empty()
}

fn schema_is_visible(
    schema: &Schema,
    prefs: &SchemaVisibilityPrefs,
    auto_hide_empty: bool,
) -> bool {
    !prefs.hidden.contains(&schema.name) && (!auto_hide_empty || schema_has_objects(schema))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use cellar_core::schema::Schema;

    use super::{schema_is_visible, SchemaVisibilityPrefs};

    #[test]
    fn visibility_respects_explicit_and_automatic_hiding() {
        let schema = Schema {
            name: "empty".into(),
            tables: vec![],
            views: vec![],
        };
        assert!(!schema_is_visible(
            &schema,
            &SchemaVisibilityPrefs::default(),
            true
        ));
        assert!(schema_is_visible(
            &schema,
            &SchemaVisibilityPrefs::default(),
            false
        ));
        assert!(!schema_is_visible(
            &schema,
            &SchemaVisibilityPrefs {
                hidden: HashSet::from(["empty".into()]),
                show_hidden: true,
            },
            false
        ));
    }
}
