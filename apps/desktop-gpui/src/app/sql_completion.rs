use anyhow::Result;
use cellar_core::schema::{Column, Database};
use gpui::{Context, Task, Window};
use gpui_component::input::{CompletionProvider, InputState, Rope, RopeExt};
use lsp_types::{
    CompletionContext, CompletionItem, CompletionItemKind, CompletionResponse, CompletionTextEdit,
    TextEdit,
};
use regex::Regex;
use std::sync::LazyLock;

static ALIAS_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b(?:from|join|update|into)\s+((?:"[^"]+"|[A-Za-z_][\w$]*)(?:\s*\.\s*(?:"[^"]+"|[A-Za-z_][\w$]*))?)(?:\s+(?:as\s+)?([A-Za-z_][\w$]*))?"#)
        .expect("static SQL alias regex")
});

#[derive(Clone)]
struct Relation {
    schema: String,
    name: String,
    columns: Vec<Column>,
    kind: CompletionItemKind,
}

pub(super) struct SqlCompletionProvider {
    relations: Vec<Relation>,
}

impl SqlCompletionProvider {
    pub(super) fn new(databases: &[Database], database: &str) -> Self {
        let selected = databases
            .iter()
            .find(|item| item.name == database)
            .or_else(|| databases.iter().find(|item| item.is_default))
            .or_else(|| databases.first());
        let relations = selected
            .into_iter()
            .flat_map(|database| &database.schemas)
            .flat_map(|schema| {
                schema
                    .tables
                    .iter()
                    .map(|table| Relation {
                        schema: schema.name.clone(),
                        name: table.name.clone(),
                        columns: table.columns.clone(),
                        kind: CompletionItemKind::CLASS,
                    })
                    .chain(schema.views.iter().map(|view| Relation {
                        schema: schema.name.clone(),
                        name: view.name.clone(),
                        columns: view.columns.clone(),
                        kind: CompletionItemKind::INTERFACE,
                    }))
            })
            .collect();
        Self { relations }
    }

    fn items(&self, sql: &str, offset: usize) -> Vec<CompletionItem> {
        let before = &sql[..offset.min(sql.len())];
        let token_start = before
            .char_indices()
            .rev()
            .find(|(_, ch)| !ch.is_ascii_alphanumeric() && !matches!(ch, '_' | '$' | '.' | '"'))
            .map_or(0, |(index, ch)| index + ch.len_utf8());
        let token = &before[token_start..];
        let qualifier = token
            .rfind('.')
            .map(|dot| normalize(&token[..dot]))
            .filter(|value| !value.is_empty());
        let statement_start = before.rfind(';').map_or(0, |index| index + 1);
        let statement_end = sql[offset..]
            .find(';')
            .map_or(sql.len(), |index| offset + index);
        let statement = &sql[statement_start..statement_end];
        let aliases = aliases(statement, &self.relations);
        if let Some(qualifier) = qualifier {
            if let Some(relation) = aliases
                .iter()
                .find_map(|(alias, relation)| (alias == &qualifier).then_some(*relation))
                .or_else(|| {
                    self.relations
                        .iter()
                        .find(|relation| normalize(&relation.name) == qualifier)
                })
            {
                return columns(&relation.columns);
            }
            let scoped = self
                .relations
                .iter()
                .filter(|relation| normalize(&relation.schema) == qualifier)
                .collect::<Vec<_>>();
            if !scoped.is_empty() {
                return relations(scoped);
            }
        }

        let previous = before[..token_start]
            .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
            .next_back()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if [
            "from", "join", "into", "update", "table", "describe", "truncate",
        ]
        .contains(&previous.as_str())
        {
            let mut items = relations(self.relations.iter().collect());
            items.extend(schemas(&self.relations));
            items.extend(keywords());
            return items;
        }

        let mut items = snippets();
        let visible_columns: Vec<Column> = if aliases.is_empty() {
            self.relations
                .iter()
                .flat_map(|relation| relation.columns.clone())
                .collect()
        } else {
            let qualify = aliases.len() > 1;
            aliases
                .iter()
                .flat_map(|(alias, relation)| {
                    relation.columns.iter().cloned().map(move |mut column| {
                        if qualify {
                            column.name = format!("{alias}.{}", column.name);
                        }
                        column
                    })
                })
                .collect()
        };
        items.extend(columns(&visible_columns));
        items.extend(relations(self.relations.iter().collect()));
        items.extend(keywords());
        items
    }
}

impl CompletionProvider for SqlCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _: CompletionContext,
        _: &mut Window,
        _: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let sql = text.to_string();
        let start = replacement_start(&sql, offset);
        let range = lsp_types::Range {
            start: text.offset_to_position(start),
            end: text.offset_to_position(offset),
        };
        let mut items = self.items(&sql, offset);
        for item in &mut items {
            let new_text = item
                .insert_text
                .take()
                .unwrap_or_else(|| item.label.clone());
            item.text_edit = Some(CompletionTextEdit::Edit(TextEdit { range, new_text }));
        }
        Task::ready(Ok(CompletionResponse::Array(items)))
    }

    fn is_completion_trigger(&self, _: usize, text: &str, _: &mut Context<InputState>) -> bool {
        text.chars()
            .last()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$' | '.'))
    }
}

fn item(label: impl Into<String>, kind: CompletionItemKind) -> CompletionItem {
    CompletionItem {
        label: label.into(),
        kind: Some(kind),
        ..Default::default()
    }
}

fn columns(source: &[Column]) -> Vec<CompletionItem> {
    let mut seen = std::collections::HashSet::new();
    source
        .iter()
        .filter(|column| seen.insert(column.name.to_ascii_lowercase()))
        .map(|column| CompletionItem {
            detail: Some(column.data_type.clone()),
            ..item(&column.name, CompletionItemKind::FIELD)
        })
        .collect()
}

fn relations(source: Vec<&Relation>) -> Vec<CompletionItem> {
    source
        .into_iter()
        .flat_map(|relation| {
            [
                CompletionItem {
                    detail: Some(if relation.kind == CompletionItemKind::INTERFACE {
                        "view".into()
                    } else {
                        "table".into()
                    }),
                    ..item(&relation.name, relation.kind)
                },
                item(
                    format!("{}.{}", relation.schema, relation.name),
                    relation.kind,
                ),
            ]
        })
        .collect()
}

fn schemas(relations: &[Relation]) -> Vec<CompletionItem> {
    let mut seen = std::collections::HashSet::new();
    relations
        .iter()
        .filter(|relation| seen.insert(relation.schema.to_ascii_lowercase()))
        .map(|relation| item(&relation.schema, CompletionItemKind::MODULE))
        .collect()
}

fn keywords() -> Vec<CompletionItem> {
    [
        "SELECT",
        "FROM",
        "WHERE",
        "JOIN",
        "LEFT JOIN",
        "INNER JOIN",
        "GROUP BY",
        "ORDER BY",
        "HAVING",
        "LIMIT",
        "INSERT INTO",
        "UPDATE",
        "DELETE FROM",
        "CREATE TABLE",
        "ALTER TABLE",
        "DROP TABLE",
        "WITH",
        "VALUES",
        "RETURNING",
        "EXPLAIN",
    ]
    .into_iter()
    .map(|keyword| item(keyword, CompletionItemKind::KEYWORD))
    .collect()
}

fn snippets() -> Vec<CompletionItem> {
    [
        ("sel", "SELECT *\nFROM \nLIMIT 100;"),
        ("ins", "INSERT INTO \n  ()\nVALUES\n  ();"),
        ("upd", "UPDATE \nSET \nWHERE ;"),
        ("del", "DELETE FROM \nWHERE ;"),
        ("jln", "LEFT JOIN  ON "),
    ]
    .into_iter()
    .map(|(label, body)| CompletionItem {
        insert_text: Some(body.into()),
        ..item(label, CompletionItemKind::SNIPPET)
    })
    .collect()
}

fn aliases<'a>(statement: &str, relations: &'a [Relation]) -> Vec<(String, &'a Relation)> {
    ALIAS_PATTERN
        .captures_iter(statement)
        .flat_map(|capture| {
            let raw = capture.get(1)?.as_str();
            let name = normalize(raw.rsplit('.').next()?);
            let relation = relations
                .iter()
                .find(|relation| normalize(&relation.name) == name)?;
            let alias = capture
                .get(2)
                .map(|alias| normalize(alias.as_str()))
                .filter(|alias| {
                    ![
                        "on",
                        "where",
                        "join",
                        "left",
                        "right",
                        "inner",
                        "outer",
                        "full",
                        "cross",
                        "group",
                        "order",
                        "limit",
                        "having",
                        "set",
                        "values",
                        "returning",
                    ]
                    .contains(&alias.as_str())
                });
            Some(
                std::iter::once((name.clone(), relation)).chain(
                    alias
                        .filter(|alias| alias != &name)
                        .map(|alias| (alias, relation)),
                ),
            )
        })
        .flatten()
        .collect()
}

fn normalize(value: &str) -> String {
    value.trim().trim_matches('"').to_ascii_lowercase()
}

fn replacement_start(sql: &str, offset: usize) -> usize {
    let before = &sql[..offset.min(sql.len())];
    let token_start = before
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_ascii_alphanumeric() && !matches!(ch, '_' | '$' | '.' | '"'))
        .map_or(0, |(index, ch)| index + ch.len_utf8());
    before[token_start..]
        .rfind('.')
        .map_or(token_start, |dot| token_start + dot + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualifier_completion_is_scoped_to_the_relation() {
        let provider = SqlCompletionProvider {
            relations: vec![Relation {
                schema: "public".into(),
                name: "customers".into(),
                columns: vec![Column {
                    name: "email".into(),
                    data_type: "text".into(),
                    nullable: false,
                    default: None,
                    is_primary_key: false,
                    ordinal: 1,
                    comment: None,
                }],
                kind: CompletionItemKind::CLASS,
            }],
        };
        let items = provider.items("SELECT c. FROM public.customers c", 9);
        assert_eq!(
            items
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            ["email"]
        );
    }
}
