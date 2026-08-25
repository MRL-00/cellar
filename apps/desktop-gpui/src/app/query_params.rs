use std::collections::HashSet;

use cellar_core::{query::DetectedParameter, value::CellValue};
use gpui::{Context, Entity};
use gpui_component::input::InputState;

use super::CellarApp;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ParamKind {
    Text,
    Number,
    Boolean,
    Date,
    Null,
}

pub(super) struct QueryParameterInput {
    pub(super) parameter: DetectedParameter,
    pub(super) kind: ParamKind,
    pub(super) state: Entity<InputState>,
}

impl ParamKind {
    pub(super) fn next(self) -> Self {
        match self {
            Self::Text => Self::Number,
            Self::Number => Self::Boolean,
            Self::Boolean => Self::Date,
            Self::Date => Self::Null,
            Self::Null => Self::Text,
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Number => "Number",
            Self::Boolean => "Boolean",
            Self::Date => "Date",
            Self::Null => "NULL",
        }
    }
}

pub(super) fn infer_param_kind(
    parameter: &DetectedParameter,
    databases: &[cellar_core::schema::Database],
    database: Option<&str>,
) -> ParamKind {
    let Some(hint) = parameter.column_hint.as_deref() else {
        return ParamKind::Text;
    };
    let mut kinds = HashSet::new();
    for db in databases
        .iter()
        .filter(|db| database.is_none_or(|name| db.name == name))
    {
        for schema in &db.schemas {
            for table in &schema.tables {
                for column in &table.columns {
                    if column.name.eq_ignore_ascii_case(hint) {
                        kinds.insert(param_kind_for_type(&column.data_type));
                    }
                }
            }
        }
    }
    (kinds.len() == 1)
        .then(|| *kinds.iter().next().expect("one inferred kind"))
        .unwrap_or(ParamKind::Text)
}

fn param_kind_for_type(data_type: &str) -> ParamKind {
    let kind = data_type.to_ascii_lowercase();
    if matches!(kind.as_str(), "bool" | "boolean") {
        ParamKind::Boolean
    } else if kind == "date" {
        ParamKind::Date
    } else if [
        "int", "serial", "oid", "float", "double", "real", "numeric", "decimal", "money",
    ]
    .iter()
    .any(|needle| kind.contains(needle))
        && !kind.contains("interval")
    {
        ParamKind::Number
    } else {
        ParamKind::Text
    }
}

pub(super) fn parameter_value(
    input: &QueryParameterInput,
    cx: &Context<CellarApp>,
) -> Result<CellValue, String> {
    let raw = input.state.read(cx).value().to_string();
    match input.kind {
        ParamKind::Text if raw.is_empty() => Err(format!(
            "Enter a value for {} or choose NULL",
            input.parameter.placeholder
        )),
        ParamKind::Text => Ok(CellValue::Text(raw)),
        ParamKind::Number => {
            let value = raw.trim();
            if let Ok(value) = value.parse::<i64>() {
                Ok(CellValue::Int(value))
            } else {
                value
                    .parse::<f64>()
                    .map(CellValue::Float)
                    .map_err(|_| format!("{} needs a valid number", input.parameter.placeholder))
            }
        }
        ParamKind::Boolean => raw
            .trim()
            .parse::<bool>()
            .map(CellValue::Bool)
            .map_err(|_| format!("{} must be true or false", input.parameter.placeholder)),
        ParamKind::Date => chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d")
            .map(CellValue::Date)
            .map_err(|_| format!("{} must use YYYY-MM-DD", input.parameter.placeholder)),
        ParamKind::Null => Ok(CellValue::Null),
    }
}

#[cfg(test)]
mod tests {
    use super::{param_kind_for_type, ParamKind};

    #[test]
    fn infers_safe_parameter_kinds_from_database_types() {
        assert_eq!(param_kind_for_type("int8"), ParamKind::Number);
        assert_eq!(param_kind_for_type("boolean"), ParamKind::Boolean);
        assert_eq!(param_kind_for_type("date"), ParamKind::Date);
        assert_eq!(param_kind_for_type("interval"), ParamKind::Text);
    }
}
