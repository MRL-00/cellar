use std::collections::HashMap;

use cellar_core::schema::Table;
use cellar_diff::{CellAssignment, DiffColumn, DiffValue, RowChange, TableChangeRequest};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportMode {
    Update,
    Insert,
    Upsert,
}

impl ImportMode {
    pub fn next(self) -> Self {
        match self {
            Self::Update => Self::Insert,
            Self::Insert => Self::Upsert,
            Self::Upsert => Self::Update,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Update => "Update only",
            Self::Insert => "Insert only",
            Self::Upsert => "Upsert",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedCsv {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

#[derive(Clone, Debug)]
pub struct ImportConfig {
    pub mapping: HashMap<String, usize>,
    pub match_keys: Vec<String>,
    pub mode: ImportMode,
    pub update_fields: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImportCounts {
    pub total: usize,
    pub to_write: usize,
    pub skipped: usize,
}

pub fn parse_csv(text: &str) -> Result<ParsedCsv, String> {
    let delimiter = sniff_delimiter(text);
    let mut rows = Vec::<Vec<Option<String>>>::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut in_quotes = false;
    let mut started = false;
    let mut chars = text.chars().peekable();

    while let Some(character) = chars.next() {
        if in_quotes {
            if character == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(character);
            }
            continue;
        }
        if character == '"' && !started {
            quoted = true;
            in_quotes = true;
            started = true;
        } else if character == delimiter {
            finish_field(&mut row, &mut field, &mut quoted, &mut started);
        } else if character == '\n' {
            finish_row(&mut rows, &mut row, &mut field, &mut quoted, &mut started);
        } else if character != '\r' {
            field.push(character);
            started = true;
        }
    }
    if in_quotes {
        return Err("CSV contains an unterminated quoted field".into());
    }
    if started || quoted || !field.is_empty() || !row.is_empty() {
        finish_row(&mut rows, &mut row, &mut field, &mut quoted, &mut started);
    }
    let Some(header) = rows.first().cloned() else {
        return Err("CSV has no header row".into());
    };
    let headers = header
        .into_iter()
        .map(|header| header.unwrap_or_default().trim().to_owned())
        .collect::<Vec<_>>();
    if headers.is_empty() || headers.iter().all(String::is_empty) {
        return Err("CSV has no named columns".into());
    }
    Ok(ParsedCsv {
        headers,
        rows: rows.into_iter().skip(1).collect(),
    })
}

pub fn default_config(csv: &ParsedCsv, table: &Table) -> ImportConfig {
    let lower = csv
        .headers
        .iter()
        .map(|header| header.to_lowercase())
        .collect::<Vec<_>>();
    let mapping = table
        .columns
        .iter()
        .filter_map(|column| {
            csv.headers
                .iter()
                .position(|header| header == &column.name)
                .or_else(|| {
                    lower
                        .iter()
                        .position(|header| header == &column.name.to_lowercase())
                })
                .map(|index| (column.name.clone(), index))
        })
        .collect::<HashMap<_, _>>();
    let match_keys = table
        .primary_key
        .iter()
        .filter(|column| mapping.contains_key(*column))
        .cloned()
        .collect::<Vec<_>>();
    let update_fields = table
        .columns
        .iter()
        .map(|column| column.name.clone())
        .filter(|column| mapping.contains_key(column) && !match_keys.contains(column))
        .collect();
    ImportConfig {
        mapping,
        match_keys,
        mode: ImportMode::Upsert,
        update_fields,
    }
}

pub fn validate_import(csv: &ParsedCsv, table: &Table, config: &ImportConfig) -> Vec<String> {
    let mut errors = Vec::new();
    if csv.rows.is_empty() {
        errors.push("The CSV has no data rows".into());
    }
    if config.mode != ImportMode::Insert && config.match_keys.is_empty() {
        errors.push("Select at least one match-key column".into());
    }
    if config.mode != ImportMode::Insert {
        for key in &config.match_keys {
            if !config.mapping.contains_key(key) {
                errors.push(format!("Match key \"{key}\" is not mapped"));
            }
        }
    }
    if matches!(config.mode, ImportMode::Update | ImportMode::Upsert)
        && !config
            .update_fields
            .iter()
            .any(|field| !config.match_keys.contains(field))
    {
        errors.push("Select at least one field to update".into());
    }
    if matches!(config.mode, ImportMode::Insert | ImportMode::Upsert) {
        for column in &table.columns {
            if !column.nullable
                && column.default.is_none()
                && !config.mapping.contains_key(&column.name)
            {
                errors.push(format!(
                    "Required column \"{}\" must be mapped",
                    column.name
                ));
            }
        }
    }
    errors
}

pub fn import_counts(csv: &ParsedCsv, config: &ImportConfig) -> ImportCounts {
    let skipped = if config.mode == ImportMode::Insert {
        0
    } else {
        csv.rows
            .iter()
            .filter(|row| row_key_missing(row, config))
            .count()
    };
    ImportCounts {
        total: csv.rows.len(),
        to_write: csv.rows.len() - skipped,
        skipped,
    }
}

pub fn build_import_request(
    database: &str,
    table: &Table,
    csv: &ParsedCsv,
    config: &ImportConfig,
) -> TableChangeRequest {
    let mapped = table
        .columns
        .iter()
        .filter(|column| config.mapping.contains_key(&column.name))
        .map(|column| column.name.clone())
        .collect::<Vec<_>>();
    let updates = config
        .update_fields
        .iter()
        .filter(|column| !config.match_keys.contains(column))
        .cloned()
        .collect::<Vec<_>>();
    let changes = csv
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| config.mode == ImportMode::Insert || !row_key_missing(row, config))
        .map(|(index, row)| {
            let row_id = format!("csv:{}", index + 2);
            match config.mode {
                ImportMode::Update => RowChange::Update {
                    row_id,
                    keys: config
                        .match_keys
                        .iter()
                        .map(|column| assignment(row, column, config))
                        .collect(),
                    edits: updates
                        .iter()
                        .map(|column| assignment(row, column, config))
                        .collect(),
                },
                ImportMode::Insert => RowChange::Insert {
                    row_id,
                    values: mapped
                        .iter()
                        .map(|column| assignment(row, column, config))
                        .collect(),
                },
                ImportMode::Upsert => RowChange::Upsert {
                    row_id,
                    conflict_columns: config.match_keys.clone(),
                    values: mapped
                        .iter()
                        .map(|column| assignment(row, column, config))
                        .collect(),
                    update_columns: updates.clone(),
                },
            }
        })
        .collect();
    TableChangeRequest {
        database: Some(database.to_owned()),
        schema: table.schema.clone(),
        table: table.name.clone(),
        primary_key: if config.mode == ImportMode::Insert {
            Vec::new()
        } else {
            config.match_keys.clone()
        },
        columns: table
            .columns
            .iter()
            .map(|column| DiffColumn {
                name: column.name.clone(),
                data_type: column.data_type.clone(),
                nullable: column.nullable,
            })
            .collect(),
        changes,
    }
}

fn sniff_delimiter(text: &str) -> char {
    let delimiters = [',', '\t', ';'];
    let mut counts = [0usize; 3];
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
        } else if !in_quotes {
            if character == '\n' {
                break;
            }
            if let Some(index) = delimiters
                .iter()
                .position(|delimiter| *delimiter == character)
            {
                counts[index] += 1;
            }
        }
    }
    let mut best = 0;
    for index in 1..delimiters.len() {
        if counts[index] > counts[best] {
            best = index;
        }
    }
    delimiters[best]
}

fn finish_field(
    row: &mut Vec<Option<String>>,
    field: &mut String,
    quoted: &mut bool,
    started: &mut bool,
) {
    row.push(if *quoted {
        Some(std::mem::take(field))
    } else if field.is_empty() {
        None
    } else {
        Some(std::mem::take(field))
    });
    field.clear();
    *quoted = false;
    *started = false;
}

fn finish_row(
    rows: &mut Vec<Vec<Option<String>>>,
    row: &mut Vec<Option<String>>,
    field: &mut String,
    quoted: &mut bool,
    started: &mut bool,
) {
    finish_field(row, field, quoted, started);
    rows.push(std::mem::take(row));
}

fn row_key_missing(row: &[Option<String>], config: &ImportConfig) -> bool {
    config.match_keys.iter().any(|key| {
        config
            .mapping
            .get(key)
            .and_then(|index| row.get(*index))
            .and_then(Option::as_deref)
            .is_none_or(str::is_empty)
    })
}

fn assignment(row: &[Option<String>], column: &str, config: &ImportConfig) -> CellAssignment {
    CellAssignment {
        column: column.to_owned(),
        value: DiffValue {
            value: config
                .mapping
                .get(column)
                .and_then(|index| row.get(*index))
                .cloned()
                .flatten(),
        },
    }
}

#[cfg(test)]
mod tests {
    use cellar_core::schema::{Column, Table};
    use cellar_diff::RowChange;

    use super::{
        build_import_request, default_config, import_counts, parse_csv, validate_import, ImportMode,
    };

    fn table() -> Table {
        Table {
            name: "users".into(),
            schema: "public".into(),
            columns: vec![
                Column {
                    name: "id".into(),
                    data_type: "int8".into(),
                    nullable: false,
                    default: None,
                    is_primary_key: true,
                    ordinal: 1,
                    comment: None,
                },
                Column {
                    name: "note".into(),
                    data_type: "text".into(),
                    nullable: true,
                    default: None,
                    is_primary_key: false,
                    ordinal: 2,
                    comment: None,
                },
            ],
            primary_key: vec!["id".into()],
            foreign_keys: Vec::new(),
            indexes: Vec::new(),
            row_count: None,
        }
    }

    #[test]
    fn parses_quotes_nulls_and_builds_typed_upserts() {
        let csv = parse_csv("id,note\r\n1,\r\n2,\"\"\r\n3,\"say \"\"hi\"\"\"").unwrap();
        assert_eq!(csv.rows[0][1], None);
        assert_eq!(csv.rows[1][1].as_deref(), Some(""));
        assert_eq!(csv.rows[2][1].as_deref(), Some("say \"hi\""));
        let config = default_config(&csv, &table());
        assert_eq!(config.mode, ImportMode::Upsert);
        let request = build_import_request("cellar", &table(), &csv, &config);
        assert_eq!(request.changes.len(), 3);
        assert!(matches!(request.changes[0], RowChange::Upsert { .. }));
    }

    #[test]
    fn insert_only_allows_generated_keys_omitted_from_csv() {
        let mut table = table();
        table.columns[0].default = Some("generated always as identity".into());
        let csv = parse_csv("note\nhello\nworld").unwrap();
        let mut config = default_config(&csv, &table);
        config.mode = ImportMode::Insert;
        assert!(validate_import(&csv, &table, &config).is_empty());
        assert_eq!(import_counts(&csv, &config).to_write, 2);
        let request = build_import_request("cellar", &table, &csv, &config);
        assert!(request.primary_key.is_empty());
        assert!(request
            .changes
            .iter()
            .all(|change| matches!(change, RowChange::Insert { .. })));
        cellar_diff::build_postgres_plan(&request).expect("plain inserts do not need row keys");
    }

    #[test]
    fn delimiter_detection_ignores_characters_inside_quoted_headers() {
        let csv = parse_csv("\"Last, First, Display\";id\nAlice;1").unwrap();
        assert_eq!(csv.headers, ["Last, First, Display", "id"]);
        assert_eq!(csv.rows[0][0].as_deref(), Some("Alice"));
    }
}
