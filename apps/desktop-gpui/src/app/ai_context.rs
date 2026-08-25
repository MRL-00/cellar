use cellar_core::schema::Table;
use gpui::Context;

use super::{ai::AiTopic, CellarApp};
use cellar_desktop_gpui::model::TabKind;

#[derive(Clone, Debug)]
pub(super) struct AiContextChip {
    pub kind: &'static str,
    pub value: String,
}

impl CellarApp {
    pub(super) fn ai_context(&self, cx: &Context<Self>) -> (String, Vec<AiContextChip>) {
        let Some(tab) = self.model.active_tab() else {
            return (String::new(), Vec::new());
        };
        let (connection_id, database, focused, query) = match &tab.kind {
            TabKind::Table { target, .. } => (
                target.connection_id.as_str(),
                target.database.as_str(),
                Some((target.schema.as_str(), target.table.as_str())),
                None,
            ),
            TabKind::Query { target, .. } => (
                target.connection_id.as_str(),
                target.database.as_str(),
                None,
                self.editors
                    .get(&tab.id)
                    .map(|editor| editor.read(cx).value().to_string()),
            ),
            TabKind::ErDiagram { target, .. } => (
                target.connection_id.as_str(),
                target.database.as_str(),
                None,
                None,
            ),
            TabKind::SchemaCompare { .. } => return (String::new(), Vec::new()),
        };
        let Some(connection) = self
            .model
            .connections()
            .iter()
            .find(|connection| connection.id == connection_id)
        else {
            return (String::new(), Vec::new());
        };
        let Some(database_meta) = self
            .model
            .databases(connection_id)
            .iter()
            .find(|item| item.name == database)
        else {
            return (String::new(), Vec::new());
        };
        let mut tables = Vec::<&Table>::new();
        if let Some((schema, table)) = focused {
            if let Some(found) = database_meta
                .schemas
                .iter()
                .find(|item| item.name == schema)
                .and_then(|item| item.tables.iter().find(|item| item.name == table))
            {
                tables.push(found);
            }
        } else {
            let refs = query
                .as_deref()
                .map(extract_relation_names)
                .unwrap_or_default();
            for schema in &database_meta.schemas {
                if is_noise_schema(&schema.name) {
                    continue;
                }
                for table in &schema.tables {
                    if refs.is_empty()
                        || refs
                            .iter()
                            .any(|name| name.eq_ignore_ascii_case(&table.name))
                    {
                        tables.push(table);
                    }
                    if tables.len() == 12 {
                        break;
                    }
                }
                if tables.len() == 12 {
                    break;
                }
            }
            if tables.is_empty() {
                tables.extend(
                    database_meta
                        .schemas
                        .iter()
                        .filter(|schema| !is_noise_schema(&schema.name))
                        .flat_map(|schema| &schema.tables)
                        .take(12),
                );
            }
        }
        let schema = focused
            .map(|(schema, _)| schema)
            .or_else(|| tables.first().map(|table| table.schema.as_str()));
        let mut context = format!(
            "Engine: {}\nConnection: {}\nDatabase: {}",
            connection.engine.as_str(),
            connection.name,
            database
        );
        if let Some(schema) = schema {
            context.push_str(&format!("\nSchema: {schema}"));
        }
        for table in &tables {
            context.push_str(&format!("\n\n{}.{} (\n", table.schema, table.name));
            for (index, column) in table.columns.iter().enumerate() {
                if index > 0 {
                    context.push_str(",\n");
                }
                context.push_str(&format!("  {} {}", column.name, column.data_type));
                if column.is_primary_key {
                    context.push_str(" [pk]");
                }
                if !column.nullable {
                    context.push_str(" [not null]");
                }
            }
            for foreign_key in &table.foreign_keys {
                context.push_str(&format!(
                    "\n  FK {}.{}({}) -> {}.{}({})",
                    table.schema,
                    table.name,
                    foreign_key.columns.join(", "),
                    foreign_key.referenced_schema,
                    foreign_key.referenced_table,
                    foreign_key.referenced_columns.join(", ")
                ));
            }
            context.push_str("\n)");
        }
        let mut chips = Vec::new();
        if let Some(schema) = schema {
            chips.push(AiContextChip {
                kind: "schema",
                value: schema.into(),
            });
        }
        if let Some((schema, table)) = focused {
            chips.push(AiContextChip {
                kind: "table",
                value: format!("{schema}.{table}"),
            });
        }
        if query.as_deref().is_some_and(|sql| !sql.trim().is_empty()) {
            chips.push(AiContextChip {
                kind: "query",
                value: tab.title.clone(),
            });
        }
        (context, chips)
    }

    pub(super) fn build_ai_prompt(&self, topic: AiTopic, text: &str, cx: &Context<Self>) -> String {
        let instruction = match topic {
            AiTopic::Generate => "Generate a SQL query for the request. Return one ```sql block, then one sentence explaining it. Use only the schema context.",
            AiTopic::Explain => "Explain the SQL or answer the data question with one read-only SQL query in a ```sql block. Use only the schema context.",
            AiTopic::Optimize => "Optimize the SQL. Return an improved ```sql block and a short list of changes while preserving the result set.",
            AiTopic::Migrate => "Draft the SQL migration in a ```sql block. Flag destructive or irreversible steps and transaction safety.",
            AiTopic::Ask => "",
        };
        let (context, _) = self.ai_context(cx);
        let now = chrono::Local::now();
        [
            (!instruction.is_empty()).then_some(instruction.to_owned()),
            Some(format!(
                "Today: {} ({}). Current year: {}.",
                now.format("%Y-%m-%d"),
                now.format("%A"),
                now.format("%Y")
            )),
            (!context.is_empty()).then_some(format!("Schema context:\n{context}")),
            Some(format!("Request:\n{}", text.trim())),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n\n")
    }
}

fn is_noise_schema(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name == "information_schema"
        || name == "pg_catalog"
        || name == "pg_toast"
        || name == "sys"
        || name.starts_with("pg_temp_")
        || name.starts_with("pg_toast_temp_")
}

fn extract_relation_names(sql: &str) -> Vec<String> {
    let tokens = sql
        .split(|character: char| {
            character.is_whitespace() || matches!(character, ',' | ';' | '(' | ')')
        })
        .collect::<Vec<_>>();
    tokens
        .windows(2)
        .filter_map(|pair| {
            matches!(
                pair[0].to_ascii_lowercase().as_str(),
                "from" | "join" | "update" | "into" | "table" | "merge"
            )
            .then(|| {
                pair[1]
                    .trim_matches(|character| matches!(character, '"' | '`' | '[' | ']'))
                    .rsplit('.')
                    .next()
                    .unwrap_or_default()
                    .to_owned()
            })
        })
        .filter(|name| !name.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::extract_relation_names;

    #[test]
    fn context_pins_relations_from_query_clauses() {
        assert_eq!(
            extract_relation_names("select * from dbo.Customers c join Orders o on 1=1"),
            ["Customers", "Orders"]
        );
    }
}
