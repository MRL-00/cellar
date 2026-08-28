use std::{
    collections::HashMap,
    io::{BufWriter, Write},
    path::Path,
};

use cellar_core::{
    error::{CellarError, CellarResult},
    query::QueryResult,
    value::CellValue,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Tsv,
    Json,
    Sql,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Json => "json",
            Self::Sql => "sql",
        }
    }
}

pub fn export_result_to_path(
    path: &Path,
    result: &QueryResult,
    format: ExportFormat,
    table: Option<(&str, &str)>,
) -> CellarResult<()> {
    write_with_temp(path, |writer| export_result(writer, result, format, table))
}

pub fn write_atomically(path: &Path, contents: &[u8]) -> CellarResult<()> {
    write_with_temp(path, |writer| {
        writer.write_all(contents).map_err(CellarError::from)
    })
}

pub fn export_result(
    writer: &mut dyn Write,
    result: &QueryResult,
    format: ExportFormat,
    table: Option<(&str, &str)>,
) -> CellarResult<()> {
    match format {
        ExportFormat::Csv => write_delimited(writer, result, b','),
        ExportFormat::Tsv => write_delimited(writer, result, b'\t'),
        ExportFormat::Json => write_json(writer, result),
        ExportFormat::Sql => write_sql(writer, result, table),
    }
}

fn write_delimited(
    writer: &mut dyn Write,
    result: &QueryResult,
    delimiter: u8,
) -> CellarResult<()> {
    for (index, column) in result.columns.iter().enumerate() {
        if index > 0 {
            writer.write_all(&[delimiter])?;
        }
        write_delimited_text(writer, &column.name, delimiter, false)?;
    }
    writer.write_all(b"\r\n")?;
    for row in &result.rows {
        for index in 0..result.columns.len() {
            if index > 0 {
                writer.write_all(&[delimiter])?;
            }
            if let Some(value) = row.get(index) {
                write_delimited_cell(writer, value, delimiter)?;
            }
        }
        writer.write_all(b"\r\n")?;
    }
    Ok(())
}

fn write_delimited_cell(
    writer: &mut dyn Write,
    value: &CellValue,
    delimiter: u8,
) -> CellarResult<()> {
    if value.is_null() {
        return Ok(());
    }
    let text = cell_text(value);
    write_delimited_text(writer, &text, delimiter, text.is_empty())
}

fn write_delimited_text(
    writer: &mut dyn Write,
    text: &str,
    delimiter: u8,
    force_quotes: bool,
) -> CellarResult<()> {
    let quoted = force_quotes
        || text
            .bytes()
            .any(|byte| byte == delimiter || matches!(byte, b'"' | b'\r' | b'\n'));
    if !quoted {
        writer.write_all(text.as_bytes())?;
        return Ok(());
    }
    writer.write_all(b"\"")?;
    writer.write_all(text.replace('"', "\"\"").as_bytes())?;
    writer.write_all(b"\"")?;
    Ok(())
}

fn write_json(writer: &mut dyn Write, result: &QueryResult) -> CellarResult<()> {
    let names = unique_column_names(result);
    writer.write_all(b"[\n")?;
    for (row_index, row) in result.rows.iter().enumerate() {
        if row_index > 0 {
            writer.write_all(b",\n")?;
        }
        writer.write_all(b"  {")?;
        for (column, name) in names.iter().enumerate() {
            if column > 0 {
                writer.write_all(b",")?;
            }
            writer.write_all(b"\n    ")?;
            serde_json::to_writer(&mut *writer, name)?;
            writer.write_all(b": ")?;
            write_json_cell(writer, row.get(column).unwrap_or(&CellValue::Null))?;
        }
        if !names.is_empty() {
            writer.write_all(b"\n  ")?;
        }
        writer.write_all(b"}")?;
    }
    writer.write_all(b"\n]\n")?;
    Ok(())
}

fn write_json_cell(writer: &mut dyn Write, value: &CellValue) -> CellarResult<()> {
    match value {
        CellValue::Null => writer.write_all(b"null")?,
        CellValue::Bool(value) => writer.write_all(value.to_string().as_bytes())?,
        CellValue::Int(value) => writer.write_all(value.to_string().as_bytes())?,
        CellValue::Float(value) if value.is_finite() => {
            writer.write_all(value.to_string().as_bytes())?
        }
        CellValue::Float(_) => writer.write_all(b"null")?,
        CellValue::Json(value) => serde_json::to_writer(writer, value)?,
        value => serde_json::to_writer(writer, &cell_text(value))?,
    }
    Ok(())
}

fn write_sql(
    writer: &mut dyn Write,
    result: &QueryResult,
    table: Option<(&str, &str)>,
) -> CellarResult<()> {
    let table = table
        .map(|(schema, table)| format!("{}.{}", quote_identifier(schema), quote_identifier(table)))
        .unwrap_or_else(|| quote_identifier("results"));
    let columns = result
        .columns
        .iter()
        .map(|column| quote_identifier(&column.name))
        .collect::<Vec<_>>()
        .join(", ");
    for row in &result.rows {
        write!(writer, "INSERT INTO {table} ({columns}) VALUES (")?;
        for column in 0..result.columns.len() {
            if column > 0 {
                writer.write_all(b", ")?;
            }
            write_sql_cell(writer, row.get(column).unwrap_or(&CellValue::Null))?;
        }
        writer.write_all(b");\n")?;
    }
    Ok(())
}

fn write_sql_cell(writer: &mut dyn Write, value: &CellValue) -> CellarResult<()> {
    match value {
        CellValue::Null => writer.write_all(b"NULL")?,
        CellValue::Bool(true) => writer.write_all(b"TRUE")?,
        CellValue::Bool(false) => writer.write_all(b"FALSE")?,
        CellValue::Int(value) => writer.write_all(value.to_string().as_bytes())?,
        CellValue::Float(value) if value.is_finite() => {
            writer.write_all(value.to_string().as_bytes())?
        }
        CellValue::Numeric(value) => writer.write_all(value.as_bytes())?,
        value => write!(writer, "'{}'", cell_text(value).replace('\'', "''"))?,
    }
    Ok(())
}

fn cell_text(value: &CellValue) -> String {
    match value {
        CellValue::Null => String::new(),
        CellValue::Bool(value) => value.to_string(),
        CellValue::Int(value) => value.to_string(),
        CellValue::Float(value) => value.to_string(),
        CellValue::Numeric(value) | CellValue::Text(value) => value.clone(),
        CellValue::Bytes(value) => {
            let mut text = String::from("\\x");
            for byte in value {
                text.push_str(&format!("{byte:02x}"));
            }
            text
        }
        CellValue::Json(value) => value.to_string(),
        CellValue::Uuid(value) => value.to_string(),
        CellValue::Date(value) => value.to_string(),
        CellValue::Time(value) => value.to_string(),
        CellValue::Timestamp(value) => value.to_string(),
        CellValue::TimestampTz(value) => value.to_rfc3339(),
    }
}

fn unique_column_names(result: &QueryResult) -> Vec<String> {
    let mut totals = HashMap::<&str, usize>::new();
    for column in &result.columns {
        *totals.entry(&column.name).or_default() += 1;
    }
    let mut seen = HashMap::<&str, usize>::new();
    result
        .columns
        .iter()
        .map(|column| {
            let occurrence = seen.entry(&column.name).or_default();
            *occurrence += 1;
            if totals[column.name.as_str()] > 1 && *occurrence > 1 {
                format!("{}_{}", column.name, occurrence)
            } else {
                column.name.clone()
            }
        })
        .collect()
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

fn write_with_temp(
    path: &Path,
    write: impl FnOnce(&mut dyn Write) -> CellarResult<()>,
) -> CellarResult<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut writer = BufWriter::new(temporary.as_file_mut());
        write(&mut writer)?;
        writer.flush()?;
    }
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| CellarError::Io(error.error.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use cellar_core::{
        query::{NoticeCapture, QueryResult},
        value::{CellValue, ColumnMeta},
    };

    use super::{export_result, export_result_to_path, ExportFormat};

    fn result() -> QueryResult {
        QueryResult {
            columns: ["id", "note", "id"]
                .map(|name| ColumnMeta {
                    name: name.into(),
                    data_type: "text".into(),
                    nullable: true,
                })
                .into(),
            rows: vec![vec![
                CellValue::Int(1),
                CellValue::Text("say \"hi\"".into()),
                CellValue::Null,
            ]],
            notices: Vec::new(),
            notice_capture: NoticeCapture::unsupported("test"),
            rows_affected: None,
            duration_ms: 0,
            truncated: false,
            total_rows: None,
        }
    }

    fn text(format: ExportFormat) -> String {
        let mut bytes = Vec::new();
        export_result(&mut bytes, &result(), format, Some(("audit", "users"))).unwrap();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn exports_nulls_quotes_and_duplicate_columns_without_loss() {
        assert_eq!(
            text(ExportFormat::Csv),
            "id,note,id\r\n1,\"say \"\"hi\"\"\",\r\n"
        );
        let json = text(ExportFormat::Json);
        assert!(json.contains("\"id_2\": null"));
        assert_eq!(
            text(ExportFormat::Sql),
            "INSERT INTO \"audit\".\"users\" (\"id\", \"note\", \"id\") VALUES (1, 'say \"hi\"', NULL);\n"
        );

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("result.csv");
        export_result_to_path(&path, &result(), ExportFormat::Csv, None).unwrap();
        export_result_to_path(&path, &result(), ExportFormat::Csv, None).unwrap();
        assert_eq!(
            std::fs::read_to_string(path).unwrap(),
            text(ExportFormat::Csv)
        );
    }
}
