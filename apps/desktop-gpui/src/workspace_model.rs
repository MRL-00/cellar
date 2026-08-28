use cellar_core::er::ErGraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableTarget {
    pub connection_id: String,
    pub database: String,
    pub schema: String,
    pub table: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryTarget {
    pub connection_id: String,
    pub database: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErDiagramTarget {
    pub connection_id: String,
    pub database: String,
    pub schemas: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum SchemaCompareSource {
    Live {
        connection_id: String,
        database: String,
        schema: String,
        label: Option<String>,
    },
    Snapshot {
        id: String,
        schema: String,
        label: Option<String>,
    },
}

impl SchemaCompareSource {
    pub fn schema(&self) -> &str {
        match self {
            Self::Live { schema, .. } | Self::Snapshot { schema, .. } => schema,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Live {
                database,
                schema,
                label,
                ..
            } => label
                .clone()
                .unwrap_or_else(|| format!("{database} / {schema}")),
            Self::Snapshot {
                id, schema, label, ..
            } => label.clone().unwrap_or_else(|| format!("{id} / {schema}")),
        }
    }

    pub fn live_connection_id(&self) -> Option<&str> {
        match self {
            Self::Live { connection_id, .. } => Some(connection_id),
            Self::Snapshot { .. } => None,
        }
    }

    pub fn references_connection(&self, id: &str) -> bool {
        self.live_connection_id() == Some(id)
    }

    pub fn database(&self) -> Option<&str> {
        match self {
            Self::Live { database, .. } => Some(database),
            Self::Snapshot { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SchemaCompareConfig {
    pub source: SchemaCompareSource,
    pub target: SchemaCompareSource,
}

#[derive(Debug, Clone)]
pub enum SchemaCompareState {
    Loading,
    Ready,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum ErDiagramState {
    Loading,
    Ready(ErGraph),
    Error(String),
}

#[derive(Debug, Clone)]
pub enum TableLoadState {
    Loading,
    Loaded,
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TablePage {
    pub offset: u32,
    pub limit: u32,
    pub rows: u32,
    pub total_rows: Option<u64>,
}

impl Default for TablePage {
    fn default() -> Self {
        Self {
            offset: 0,
            limit: 500,
            rows: 0,
            total_rows: None,
        }
    }
}

impl TablePage {
    pub fn has_previous(self) -> bool {
        self.offset > 0
    }

    pub fn has_next(self) -> bool {
        self.total_rows
            .map(|total| u64::from(self.offset) + u64::from(self.rows) < total)
            .unwrap_or(self.rows == self.limit)
    }
}

#[derive(Debug, Clone)]
pub enum QueryState {
    Editing,
    Running {
        rows_received: u64,
    },
    Complete {
        rows_received: u64,
        duration_ms: u64,
    },
    Error(String),
}

#[derive(Debug, Clone)]
pub enum TabKind {
    Table {
        target: TableTarget,
        state: TableLoadState,
        page: TablePage,
    },
    Query {
        target: QueryTarget,
        state: QueryState,
    },
    ErDiagram {
        target: ErDiagramTarget,
        state: ErDiagramState,
    },
    SchemaCompare {
        config: SchemaCompareConfig,
        state: SchemaCompareState,
    },
}

#[derive(Debug, Clone)]
pub struct WorkspaceTab {
    pub id: u64,
    pub title: String,
    pub pinned: bool,
    pub kind: TabKind,
}
