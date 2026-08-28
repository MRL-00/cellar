//! Binding [`CellValue`] parameters into a sqlx Postgres query.
//!
//! Every value is bound through sqlx's native parameter protocol — the SQL
//! text only ever contains `$1..$N` placeholders (produced by `cellar-sql`),
//! never the value itself. This is the SQL-injection-safe path required by the
//! security rules in `AGENTS.md`.

use std::str::FromStr;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::value::CellValue;
use sqlx::postgres::{PgArguments, PgTypeInfo, Postgres};
use sqlx::query::Query;
use sqlx::types::BigDecimal;
use sqlx::Type;

/// A sqlx query bound to the Postgres backend with positional arguments.
pub type PgQuery<'q> = Query<'q, Postgres, PgArguments>;

pub(crate) fn type_info(value: &CellValue) -> PgTypeInfo {
    fn of<T: Type<Postgres>>(_: &T) -> PgTypeInfo {
        T::type_info()
    }

    match value {
        CellValue::Null => <String as Type<Postgres>>::type_info(),
        CellValue::Bool(value) => of(value),
        CellValue::Int(value) => of(value),
        CellValue::Float(value) => of(value),
        CellValue::Numeric(_) => <BigDecimal as Type<Postgres>>::type_info(),
        CellValue::Text(value) => of(value),
        CellValue::Bytes(value) => of(value),
        CellValue::Json(value) => of(value),
        CellValue::Uuid(value) => of(value),
        CellValue::Date(value) => of(value),
        CellValue::Time(value) => of(value),
        CellValue::Timestamp(value) => of(value),
        CellValue::TimestampTz(value) => of(value),
    }
}

/// Bind one typed cell value to `query`, returning the extended query.
///
/// The value's variant chooses the Postgres wire type. Postgres compares a
/// bound value against a column using that type, so a `Text` value sent against
/// a non-text column can raise a type error; the UI infers types from column
/// context to avoid this where it can, and the user can pick a type otherwise.
pub fn bind_value<'q>(query: PgQuery<'q>, value: &'q CellValue) -> CellarResult<PgQuery<'q>> {
    Ok(match value {
        // Bind a typed NULL. `Option::<&str>::None` sends an untyped/text NULL,
        // which Postgres accepts for `IS NULL` and equality (never matches).
        CellValue::Null => query.bind(Option::<&str>::None),
        CellValue::Bool(b) => query.bind(*b),
        CellValue::Int(i) => query.bind(*i),
        CellValue::Float(f) => query.bind(*f),
        CellValue::Numeric(s) => {
            let decimal = BigDecimal::from_str(s)
                .map_err(|e| CellarError::query(format!("invalid numeric parameter `{s}`: {e}")))?;
            query.bind(decimal)
        }
        CellValue::Text(s) => query.bind(s.as_str()),
        CellValue::Bytes(b) => query.bind(b.as_slice()),
        CellValue::Json(v) => query.bind(v),
        CellValue::Uuid(u) => query.bind(*u),
        CellValue::Date(d) => query.bind(*d),
        CellValue::Time(t) => query.bind(*t),
        CellValue::Timestamp(ts) => query.bind(*ts),
        CellValue::TimestampTz(ts) => query.bind(*ts),
    })
}
