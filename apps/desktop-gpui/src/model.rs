use std::collections::{HashMap, HashSet};

use cellar_core::{
    driver::ConnectionConfig,
    er::ErGraph,
    schema::{Database, Table},
};
use serde::{Deserialize, Serialize};

mod splits;

pub use crate::workspace_model::{
    ErDiagramState, ErDiagramTarget, QueryState, QueryTarget, SchemaCompareConfig,
    SchemaCompareSource, SchemaCompareState, TabKind, TableLoadState, TablePage, TableTarget,
    WorkspaceTab,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Disconnecting,
    Connected,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SchemaNode {
    Database {
        connection_id: String,
        database: String,
    },
    Schema {
        connection_id: String,
        database: String,
        schema: String,
    },
    Group {
        connection_id: String,
        database: String,
        schema: String,
        kind: &'static str,
    },
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SplitOrientation {
    Horizontal,
    Vertical,
}

#[derive(Default)]
pub struct AppModel {
    connections: Vec<ConnectionConfig>,
    active_connection: Option<String>,
    states: HashMap<String, ConnectionState>,
    schemas: HashMap<String, Vec<Database>>,
    expanded_nodes: HashSet<SchemaNode>,
    expanded_connections: HashSet<String>,
    tabs: Vec<WorkspaceTab>,
    active_tab: Option<u64>,
    split: Option<SplitOrientation>,
    tab_panes: HashMap<u64, u8>,
    pane_active: [Option<u64>; 2],
    focused_pane: u8,
    next_tab_id: u64,
    next_query_number: u64,
    table_load_generations: HashMap<u64, u64>,
}

impl AppModel {
    pub fn new(connections: Vec<ConnectionConfig>) -> Self {
        let active_connection = connections.first().map(|config| config.id.clone());
        Self {
            states: connections
                .iter()
                .map(|config| (config.id.clone(), ConnectionState::Disconnected))
                .collect(),
            connections,
            active_connection,
            schemas: HashMap::new(),
            expanded_nodes: HashSet::new(),
            expanded_connections: HashSet::new(),
            tabs: Vec::new(),
            active_tab: None,
            split: None,
            tab_panes: HashMap::new(),
            pane_active: [None, None],
            focused_pane: 0,
            next_tab_id: 1,
            next_query_number: 1,
            table_load_generations: HashMap::new(),
        }
    }

    pub fn connections(&self) -> &[ConnectionConfig] {
        &self.connections
    }

    pub fn upsert_connection(&mut self, config: ConnectionConfig) {
        let id = config.id.clone();
        if let Some(existing) = self
            .connections
            .iter_mut()
            .find(|existing| existing.id == id)
        {
            *existing = config;
        } else {
            self.connections.push(config);
        }
        self.connections.sort_by(|a, b| a.name.cmp(&b.name));
        self.states
            .insert(id.clone(), ConnectionState::Disconnected);
        self.active_connection = Some(id);
    }

    pub fn remove_connection(&mut self, id: &str) {
        self.connections.retain(|config| config.id != id);
        self.states.remove(id);
        self.schemas.remove(id);
        self.expanded_connections.remove(id);
        self.tabs.retain(|tab| match &tab.kind {
            TabKind::Table { target, .. } => target.connection_id != id,
            TabKind::Query { target, .. } => target.connection_id != id,
            TabKind::ErDiagram { target, .. } => target.connection_id != id,
            TabKind::SchemaCompare { config, .. } => {
                !config.source.references_connection(id) && !config.target.references_connection(id)
            }
        });
        self.table_load_generations
            .retain(|tab_id, _| self.tabs.iter().any(|tab| tab.id == *tab_id));
        if self.active_connection.as_deref() == Some(id) {
            self.active_connection = self.connections.first().map(|config| config.id.clone());
        }
        if self
            .active_tab
            .is_some_and(|active| !self.tabs.iter().any(|tab| tab.id == active))
        {
            self.active_tab = self.tabs.first().map(|tab| tab.id);
        }
        self.reconcile_split();
    }

    pub fn active_connection(&self) -> Option<&ConnectionConfig> {
        let id = self.active_connection.as_deref()?;
        self.connections.iter().find(|config| config.id == id)
    }

    pub fn select_connection(&mut self, id: &str) -> bool {
        if !self.connections.iter().any(|config| config.id == id) {
            return false;
        }
        self.active_connection = Some(id.to_owned());
        true
    }

    pub fn connection_expanded(&self, id: &str) -> bool {
        self.expanded_connections.contains(id)
    }

    pub fn toggle_connection_expanded(&mut self, id: &str) -> bool {
        if !self.connections.iter().any(|config| config.id == id) {
            return false;
        }
        if self.expanded_connections.remove(id) {
            false
        } else {
            self.expanded_connections.insert(id.to_owned());
            true
        }
    }

    pub fn connection_state(&self, id: &str) -> &ConnectionState {
        self.states
            .get(id)
            .unwrap_or(&ConnectionState::Disconnected)
    }

    pub fn begin_connect(&mut self, id: &str) -> bool {
        if !self.connections.iter().any(|config| config.id == id)
            || matches!(
                self.connection_state(id),
                ConnectionState::Connecting
                    | ConnectionState::Disconnecting
                    | ConnectionState::Connected
            )
        {
            return false;
        }
        self.states
            .insert(id.to_owned(), ConnectionState::Connecting);
        true
    }

    pub fn begin_reconnect(&mut self, id: &str) -> bool {
        if !self.connections.iter().any(|config| config.id == id)
            || matches!(
                self.connection_state(id),
                ConnectionState::Connecting | ConnectionState::Disconnecting
            )
        {
            return false;
        }
        self.states
            .insert(id.to_owned(), ConnectionState::Connecting);
        true
    }

    pub fn begin_disconnect(&mut self, id: &str) -> bool {
        if !matches!(self.connection_state(id), ConnectionState::Connected) {
            return false;
        }
        self.states
            .insert(id.to_owned(), ConnectionState::Disconnecting);
        true
    }

    pub fn finish_disconnect(&mut self, id: &str) {
        self.schemas.remove(id);
        self.states
            .insert(id.to_owned(), ConnectionState::Disconnected);
    }

    pub fn finish_connect(&mut self, id: &str, result: Result<Vec<Database>, String>) {
        match result {
            Ok(databases) => {
                for database in databases.iter().filter(|database| database.is_default) {
                    self.expanded_nodes.insert(SchemaNode::Database {
                        connection_id: id.to_owned(),
                        database: database.name.clone(),
                    });
                    for schema in &database.schemas {
                        self.expanded_nodes.insert(SchemaNode::Schema {
                            connection_id: id.to_owned(),
                            database: database.name.clone(),
                            schema: schema.name.clone(),
                        });
                        for kind in ["tables", "views"] {
                            self.expanded_nodes.insert(SchemaNode::Group {
                                connection_id: id.to_owned(),
                                database: database.name.clone(),
                                schema: schema.name.clone(),
                                kind,
                            });
                        }
                    }
                }
                self.schemas.insert(id.to_owned(), databases);
                self.states
                    .insert(id.to_owned(), ConnectionState::Connected);
            }
            Err(error) => {
                self.schemas.remove(id);
                self.states
                    .insert(id.to_owned(), ConnectionState::Error(error));
            }
        }
    }

    pub fn databases(&self, id: &str) -> &[Database] {
        self.schemas.get(id).map(Vec::as_slice).unwrap_or_default()
    }

    pub fn table(&self, target: &TableTarget) -> Option<&Table> {
        self.databases(&target.connection_id)
            .iter()
            .find(|database| database.name == target.database)
            .and_then(|database| {
                database
                    .schemas
                    .iter()
                    .find(|schema| schema.name == target.schema)
            })
            .and_then(|schema| {
                schema
                    .tables
                    .iter()
                    .find(|table| table.name == target.table)
            })
    }

    pub fn toggle_node(&mut self, node: SchemaNode) {
        if !self.expanded_nodes.remove(&node) {
            self.expanded_nodes.insert(node);
        }
    }

    pub fn node_expanded(&self, node: &SchemaNode) -> bool {
        self.expanded_nodes.contains(node)
    }

    fn activate_tab(&mut self, id: u64, new: bool) {
        let pane = if self.split.is_some() {
            if new {
                let pane = self.focused_pane.min(1);
                self.tab_panes.insert(id, pane);
                pane
            } else {
                self.tab_pane(id)
            }
        } else {
            0
        };
        self.focused_pane = pane;
        self.pane_active[pane as usize] = Some(id);
        self.active_tab = Some(id);
    }

    /// Open a table or focus its existing tab. The boolean is true only when
    /// the caller must start the first page load.
    pub fn open_table(&mut self, target: TableTarget) -> (u64, bool) {
        if let Some(tab) = self
            .tabs
            .iter()
            .find(|tab| matches!(&tab.kind, TabKind::Table { target: open, .. } if open == &target))
        {
            let id = tab.id;
            self.activate_tab(id, false);
            return (id, false);
        }

        let id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(WorkspaceTab {
            id,
            title: target.table.clone(),
            pinned: false,
            kind: TabKind::Table {
                target,
                state: TableLoadState::Loading,
                page: TablePage::default(),
            },
        });
        self.activate_tab(id, true);
        (id, true)
    }

    pub fn next_table_load(&mut self, tab_id: u64) -> u64 {
        let generation = self.table_load_generations.entry(tab_id).or_default();
        *generation += 1;
        *generation
    }

    pub fn finish_table_load(
        &mut self,
        tab_id: u64,
        generation: u64,
        result: Result<(u32, Option<u64>), String>,
    ) -> bool {
        if self.table_load_generations.get(&tab_id) != Some(&generation) {
            return false;
        }
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return false;
        };
        let TabKind::Table { state, page, .. } = &mut tab.kind else {
            return false;
        };
        *state = match result {
            Ok((rows, total_rows)) => {
                page.rows = rows;
                if total_rows.is_some() {
                    page.total_rows = total_rows;
                }
                TableLoadState::Loaded
            }
            Err(error) => TableLoadState::Error(error),
        };
        true
    }

    pub fn begin_table_load(
        &mut self,
        tab_id: u64,
        offset: Option<u32>,
    ) -> Option<(TableTarget, TablePage)> {
        let tab = self.tabs.iter_mut().find(|tab| tab.id == tab_id)?;
        let TabKind::Table {
            target,
            state,
            page,
        } = &mut tab.kind
        else {
            return None;
        };
        if let Some(offset) = offset {
            page.offset = offset;
        }
        *state = TableLoadState::Loading;
        Some((target.clone(), *page))
    }

    pub fn table_page(&self, tab_id: u64) -> Option<TablePage> {
        let tab = self.tabs.iter().find(|tab| tab.id == tab_id)?;
        let TabKind::Table { page, .. } = &tab.kind else {
            return None;
        };
        Some(*page)
    }

    pub fn reset_table_page(&mut self, tab_id: u64) -> Option<(TableTarget, TablePage)> {
        let tab = self.tabs.iter_mut().find(|tab| tab.id == tab_id)?;
        let TabKind::Table {
            target,
            state,
            page,
        } = &mut tab.kind
        else {
            return None;
        };
        page.offset = 0;
        page.total_rows = None;
        *state = TableLoadState::Loading;
        Some((target.clone(), *page))
    }

    pub fn new_query(&mut self, target: QueryTarget) -> u64 {
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let number = self.next_query_number;
        self.next_query_number += 1;
        self.tabs.push(WorkspaceTab {
            id,
            title: format!("untitled-{number}.sql"),
            pinned: false,
            kind: TabKind::Query {
                target,
                state: QueryState::Editing,
            },
        });
        self.activate_tab(id, true);
        id
    }

    pub fn set_query_database(&mut self, tab_id: u64, database: String) -> bool {
        let Some(TabKind::Query { target, state }) = self
            .tabs
            .iter_mut()
            .find(|tab| tab.id == tab_id)
            .map(|tab| &mut tab.kind)
        else {
            return false;
        };
        if target.database == database {
            return false;
        }
        target.database = database;
        *state = QueryState::Editing;
        true
    }

    pub fn open_er_diagram(&mut self, target: ErDiagramTarget) -> (u64, bool) {
        if let Some(tab) = self.tabs.iter().find(
            |tab| matches!(&tab.kind, TabKind::ErDiagram { target: open, .. } if open == &target),
        ) {
            let id = tab.id;
            self.activate_tab(id, false);
            return (id, false);
        }
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let title = target
            .schemas
            .as_ref()
            .filter(|schemas| schemas.len() == 1)
            .map_or_else(
                || format!("ER: {}", target.database),
                |schemas| format!("ER: {}", schemas[0]),
            );
        self.tabs.push(WorkspaceTab {
            id,
            title,
            pinned: false,
            kind: TabKind::ErDiagram {
                target,
                state: ErDiagramState::Loading,
            },
        });
        self.activate_tab(id, true);
        (id, true)
    }

    pub fn finish_er_diagram(&mut self, tab_id: u64, result: Result<ErGraph, String>) {
        let Some(TabKind::ErDiagram { state, .. }) = self
            .tabs
            .iter_mut()
            .find(|tab| tab.id == tab_id)
            .map(|tab| &mut tab.kind)
        else {
            return;
        };
        *state = result.map_or_else(ErDiagramState::Error, ErDiagramState::Ready);
    }

    pub fn open_schema_compare(&mut self, config: SchemaCompareConfig) -> (u64, bool) {
        if let Some(tab) = self.tabs.iter().find(|tab| {
            matches!(&tab.kind, TabKind::SchemaCompare { config: open, .. } if open == &config)
        }) {
            let id = tab.id;
            self.activate_tab(id, false);
            return (id, false);
        }
        let id = self.next_tab_id;
        self.next_tab_id += 1;
        let title = format!("{} ↔ {}", config.source.schema(), config.target.schema());
        self.tabs.push(WorkspaceTab {
            id,
            title,
            pinned: false,
            kind: TabKind::SchemaCompare {
                config,
                state: SchemaCompareState::Loading,
            },
        });
        self.activate_tab(id, true);
        (id, true)
    }

    pub fn start_schema_compare(&mut self, tab_id: u64) {
        if let Some(TabKind::SchemaCompare { state, .. }) = self
            .tabs
            .iter_mut()
            .find(|tab| tab.id == tab_id)
            .map(|tab| &mut tab.kind)
        {
            *state = SchemaCompareState::Loading;
        }
    }

    pub fn finish_schema_compare(&mut self, tab_id: u64, result: Result<(), String>) {
        if let Some(TabKind::SchemaCompare { state, .. }) = self
            .tabs
            .iter_mut()
            .find(|tab| tab.id == tab_id)
            .map(|tab| &mut tab.kind)
        {
            *state = result.map_or_else(SchemaCompareState::Error, |_| SchemaCompareState::Ready);
        }
    }

    pub fn start_er_diagram(&mut self, tab_id: u64) {
        if let Some(TabKind::ErDiagram { state, .. }) = self
            .tabs
            .iter_mut()
            .find(|tab| tab.id == tab_id)
            .map(|tab| &mut tab.kind)
        {
            *state = ErDiagramState::Loading;
        }
    }

    pub fn begin_query(&mut self, tab_id: u64) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return false;
        };
        let TabKind::Query { state, .. } = &mut tab.kind else {
            return false;
        };
        if matches!(state, QueryState::Running { .. }) {
            return false;
        }
        *state = QueryState::Running { rows_received: 0 };
        true
    }

    pub fn receive_query_page(&mut self, tab_id: u64, row_count: u64) {
        let Some(WorkspaceTab {
            kind:
                TabKind::Query {
                    state: QueryState::Running { rows_received },
                    ..
                },
            ..
        }) = self.tabs.iter_mut().find(|tab| tab.id == tab_id)
        else {
            return;
        };
        *rows_received += row_count;
    }

    pub fn finish_query(&mut self, tab_id: u64, result: Result<(u64, u64), String>) {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        let TabKind::Query { state, .. } = &mut tab.kind else {
            return;
        };
        *state = match result {
            Ok((rows_received, duration_ms)) => QueryState::Complete {
                rows_received,
                duration_ms,
            },
            Err(error) => QueryState::Error(error),
        };
    }

    pub fn tabs(&self) -> &[WorkspaceTab] {
        &self.tabs
    }

    pub fn active_tab(&self) -> Option<&WorkspaceTab> {
        let id = self.active_tab?;
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn close_tab(&mut self, id: u64) {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        self.tabs.remove(index);
        self.tab_panes.remove(&id);
        if self.active_tab == Some(id) {
            self.active_tab = self
                .tabs
                .get(index.min(self.tabs.len().saturating_sub(1)))
                .map(|tab| tab.id);
        }
        self.reconcile_split();
        self.table_load_generations.remove(&id);
    }
}

#[cfg(test)]
mod tests;
