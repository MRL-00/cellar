use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use specta::Type;
use uuid::Uuid;

/// One cell value, tagged so the frontend can render the right editor and
/// preserve type fidelity through the IPC boundary.
///
/// `Numeric` is intentionally a `String`: Postgres `numeric` has arbitrary
/// precision and JavaScript `number` does not. Drivers that decode it as a
/// float would lose digits, so we round-trip it as text.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum CellValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    /// Arbitrary-precision decimal as a string to preserve precision.
    Numeric(String),
    Text(String),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
    Uuid(Uuid),
    /// Calendar date with no timezone.
    Date(NaiveDate),
    /// Wall-clock time with no date or timezone.
    Time(NaiveTime),
    /// Local timestamp without a timezone (Postgres `timestamp`).
    Timestamp(NaiveDateTime),
    /// Absolute timestamp with UTC offset (Postgres `timestamptz`).
    TimestampTz(DateTime<Utc>),
}

impl CellValue {
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

/// One row in a query result.
pub type Row = Vec<CellValue>;

/// Lightweight column descriptor for a query result. The full structural
/// schema lives in [`crate::schema::Column`]; this only carries what the grid
/// needs to render headers.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
pub struct ColumnMeta {
    pub name: String,
    /// The engine-native type name (e.g. `int4`, `text`, `jsonb`).
    pub data_type: String,
    /// Whether the column came back as a known nullable position. Result-set
    /// columns are nullable by default — drivers can sharpen this where they
    /// can prove non-null.
    pub nullable: bool,
}
