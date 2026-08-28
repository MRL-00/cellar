use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use cellar_core::query::{TableFilterClause, TableSortClause};
use gpui::{point, px, size, Bounds, Context, Pixels, Window};
use serde::{Deserialize, Serialize};

use super::CellarApp;
use cellar_desktop_gpui::{
    grid::GridLayout,
    model::{QueryTarget, SchemaCompareConfig, SplitOrientation, TabKind, TableTarget},
};

const SESSION_FILE: &str = "gpui-session.json";
const SESSION_VERSION: u8 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct SessionState {
    version: u8,
    window: SavedBounds,
    sidebar_width: f32,
    #[serde(default)]
    layout: Option<SavedLayout>,
    #[serde(default)]
    active_connection: Option<String>,
    active_tab: Option<usize>,
    tabs: Vec<SavedTab>,
    #[serde(default)]
    split: Option<SplitOrientation>,
    #[serde(default)]
    tab_panes: Vec<u8>,
    #[serde(default)]
    schema_visibility: HashMap<String, super::schema_visibility::SchemaVisibilityPrefs>,
    #[serde(default)]
    preferences: Option<super::preferences::Preferences>,
    #[serde(default)]
    sidebar_layout: Vec<super::sidebar_layout::SidebarItem>,
    #[serde(default)]
    filter_presets: HashMap<String, Vec<super::table_presets::FilterPreset>>,
    #[serde(default)]
    table_layouts: HashMap<String, GridLayout>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct SavedBounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct SavedLayout {
    left: bool,
    right: bool,
    bottom: bool,
    left_width: f32,
    right_width: f32,
    bottom_height: f32,
    #[serde(default = "default_bottom_tab")]
    bottom_tab: super::shell::BottomPanelTab,
}

fn default_bottom_tab() -> super::shell::BottomPanelTab {
    super::shell::BottomPanelTab::Results
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SavedTab {
    Table {
        target: TableTarget,
        pinned: bool,
        sort: Option<TableSortClause>,
        filters: Vec<TableFilterClause>,
        #[serde(default)]
        quick_filter: String,
        #[serde(default)]
        quick_column: Option<usize>,
        #[serde(default)]
        layout: Option<GridLayout>,
    },
    Query {
        target: QueryTarget,
        pinned: bool,
        sql: String,
        #[serde(default)]
        saved_sql: String,
        #[serde(default)]
        layout: Option<GridLayout>,
    },
    ErDiagram {
        target: cellar_desktop_gpui::model::ErDiagramTarget,
        pinned: bool,
    },
    SchemaCompare {
        config: SchemaCompareConfig,
        pinned: bool,
    },
}

impl SessionState {
    pub(crate) fn load() -> Option<Self> {
        let path = session_path()?;
        if fs::metadata(&path).ok()?.len() > 8 * 1024 * 1024 {
            return None;
        }
        let state: Self = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
        (state.version == SESSION_VERSION && state.window.valid()).then_some(state)
    }

    pub(crate) fn window_bounds(&self) -> Bounds<Pixels> {
        Bounds::new(
            point(px(self.window.x), px(self.window.y)),
            size(px(self.window.width), px(self.window.height)),
        )
    }

    pub(crate) fn empty(window: Bounds<Pixels>) -> Self {
        Self {
            version: SESSION_VERSION,
            window: SavedBounds::from_bounds(window),
            sidebar_width: 256.,
            layout: None,
            active_connection: None,
            active_tab: None,
            tabs: Vec::new(),
            split: None,
            tab_panes: Vec::new(),
            schema_visibility: HashMap::new(),
            preferences: None,
            sidebar_layout: Vec::new(),
            filter_presets: HashMap::new(),
            table_layouts: HashMap::new(),
        }
    }

    fn save(&self) -> io::Result<()> {
        let path = session_path().ok_or_else(|| io::Error::other("home directory unavailable"))?;
        save_to(
            &path,
            &serde_json::to_vec_pretty(self).map_err(io::Error::other)?,
        )
    }
}

impl SavedBounds {
    fn from_bounds(bounds: Bounds<Pixels>) -> Self {
        Self {
            x: bounds.origin.x.into(),
            y: bounds.origin.y.into(),
            width: bounds.size.width.into(),
            height: bounds.size.height.into(),
        }
    }

    fn valid(self) -> bool {
        [self.x, self.y, self.width, self.height]
            .into_iter()
            .all(f32::is_finite)
            && self.width >= 960.
            && self.height >= 600.
    }
}

impl CellarApp {
    pub(crate) fn restore_session(
        &mut self,
        session: SessionState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let restore_connection = session.active_connection.clone();
        if let Some(preferences) = session.preferences.clone() {
            self.preferences = preferences.sanitized();
        }
        self.apply_appearance(window, cx);
        self.schema_visibility = session.schema_visibility.clone();
        if !session.sidebar_layout.is_empty() {
            self.sidebar_layout = session.sidebar_layout.clone();
        }
        self.table_filter_presets = session.filter_presets.clone();
        self.table_layouts = session.table_layouts.clone();
        if let Some(layout) = session.layout {
            self.sidebar_open = layout.left;
            self.right_panel_open = layout.right;
            self.bottom_panel_open = layout.bottom;
            self.bottom_panel_tab = layout.bottom_tab;
            self.sidebar_width = layout.left_width.clamp(200., 600.);
            self.right_panel_width = layout.right_width.clamp(280., 720.);
            let max_bottom = (f32::from(self.window_bounds.size.height)
                / cellar_desktop_gpui::theme::ui_scale()
                * 0.7)
                .round()
                .max(140.);
            self.bottom_panel_height = layout.bottom_height.clamp(140., max_bottom);
        } else if !self.tauri_layout_loaded {
            self.sidebar_width = session.sidebar_width.clamp(200., 600.);
        }
        if let Some(connection_id) = session.active_connection.as_deref() {
            self.model.select_connection(connection_id);
        }
        let split = session.split;
        let saved_panes = session.tab_panes.clone();
        let mut restored_ids = Vec::with_capacity(session.tabs.len());
        for tab in session.tabs {
            let (id, pinned) = match tab {
                SavedTab::Table {
                    target,
                    pinned,
                    sort,
                    filters,
                    quick_filter,
                    quick_column,
                    layout,
                } => {
                    self.open_table(target, window, cx);
                    let id = self.model.active_tab().expect("table tab was opened").id;
                    if let Some(sort) = sort {
                        self.table_sorts.insert(id, sort);
                    }
                    if !filters.is_empty() {
                        self.table_filters.insert(id, filters);
                    }
                    if !quick_filter.is_empty() {
                        self.table_quick_filters.insert(id, quick_filter.clone());
                        if let Some(input) = self.table_quick_filter_inputs.get(&id) {
                            input.update(cx, |input, cx| input.set_value(quick_filter, window, cx));
                        }
                    }
                    if let Some(column) = quick_column {
                        self.table_quick_filter_columns.insert(id, column);
                    }
                    if let Some(layout) = layout {
                        self.grid_layouts.insert(id, layout);
                    }
                    if self.table_sorts.contains_key(&id)
                        || self.table_filters.contains_key(&id)
                        || self.table_quick_filters.contains_key(&id)
                    {
                        self.reload_table(id, cx);
                    }
                    (id, pinned)
                }
                SavedTab::Query {
                    target,
                    pinned,
                    sql,
                    saved_sql,
                    layout,
                } => {
                    let id = self.open_query(target, sql, window, cx);
                    self.query_saved_sql.insert(id, saved_sql);
                    if let Some(layout) = layout {
                        self.grid_layouts.insert(id, layout);
                    }
                    (id, pinned)
                }
                SavedTab::ErDiagram { target, pinned } => {
                    let id = self.open_er_diagram(target, cx);
                    (id, pinned)
                }
                SavedTab::SchemaCompare { config, pinned } => {
                    let id = self.open_schema_compare(config, window, cx);
                    (id, pinned)
                }
            };
            if pinned {
                self.model.toggle_tab_pin(id);
            }
            restored_ids.push(id);
        }
        self.model.restore_split(
            split,
            restored_ids.iter().copied().zip(saved_panes.into_iter()),
        );
        if let Some(id) = session
            .active_tab
            .and_then(|index| restored_ids.get(index))
            .copied()
        {
            self.model.select_tab(id);
        }
        if let Some(id) = restore_connection {
            self.start_connect(id, cx);
        }

        cx.observe_window_bounds(window, |this, window, cx| {
            this.window_bounds = window.bounds();
            if this.dismiss_context_menus() {
                cx.notify();
            }
        })
        .detach();
        cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() && this.dismiss_context_menus() {
                cx.notify();
            }
        })
        .detach();
        cx.on_app_quit(|this, cx| {
            let session = this.session_snapshot(cx);
            async move {
                let _ = session.save();
            }
        })
        .detach();
    }

    fn session_snapshot(&self, cx: &Context<Self>) -> SessionState {
        let active_id = self.model.active_tab().map(|tab| tab.id);
        let saved_tabs = self
            .model
            .tabs()
            .iter()
            .filter_map(|tab| match &tab.kind {
                TabKind::Table { target, .. } => Some((
                    tab.id,
                    SavedTab::Table {
                        target: target.clone(),
                        pinned: tab.pinned,
                        sort: self
                            .preferences
                            .grid
                            .remember_table_sort
                            .then(|| self.table_sorts.get(&tab.id).cloned())
                            .flatten(),
                        filters: self.table_filters.get(&tab.id).cloned().unwrap_or_default(),
                        quick_filter: self
                            .table_quick_filters
                            .get(&tab.id)
                            .cloned()
                            .unwrap_or_default(),
                        quick_column: self.table_quick_filter_columns.get(&tab.id).copied(),
                        layout: self.grid_layout(tab.id, cx),
                    },
                )),
                TabKind::Query { target, .. } => {
                    let sql = self.editors.get(&tab.id)?.read(cx).value().to_string();
                    let sql = cellar_runtime::history::sql_for_storage(&sql)?.to_owned();
                    let saved_sql = self
                        .query_saved_sql
                        .get(&tab.id)
                        .and_then(|sql| cellar_runtime::history::sql_for_storage(sql))
                        .unwrap_or_default()
                        .to_owned();
                    Some((
                        tab.id,
                        SavedTab::Query {
                            target: target.clone(),
                            pinned: tab.pinned,
                            sql,
                            saved_sql,
                            layout: self.grid_layout(tab.id, cx),
                        },
                    ))
                }
                TabKind::ErDiagram { target, .. } => Some((
                    tab.id,
                    SavedTab::ErDiagram {
                        target: target.clone(),
                        pinned: tab.pinned,
                    },
                )),
                TabKind::SchemaCompare { config, .. } => Some((
                    tab.id,
                    SavedTab::SchemaCompare {
                        config: config.clone(),
                        pinned: tab.pinned,
                    },
                )),
            })
            .collect::<Vec<_>>();
        let active_tab = active_id
            .and_then(|active| saved_tabs.iter().position(|(tab_id, _)| *tab_id == active));
        let tab_panes = saved_tabs
            .iter()
            .map(|(tab_id, _)| self.model.tab_pane(*tab_id))
            .collect();
        let tabs = saved_tabs.into_iter().map(|(_, tab)| tab).collect();
        let mut table_layouts = self.table_layouts.clone();
        for tab in self.model.tabs() {
            if let TabKind::Table { target, .. } = &tab.kind {
                if let Some(layout) = self.grid_layout(tab.id, cx) {
                    table_layouts.insert(super::table_workspace::table_layout_key(target), layout);
                }
            }
        }
        SessionState {
            version: SESSION_VERSION,
            window: SavedBounds::from_bounds(self.window_bounds),
            sidebar_width: self.sidebar_width,
            layout: Some(SavedLayout {
                left: self.sidebar_open,
                right: self.right_panel_open,
                bottom: self.bottom_panel_open,
                left_width: self.sidebar_width,
                right_width: self.right_panel_width,
                bottom_height: self.bottom_panel_height,
                bottom_tab: self.bottom_panel_tab,
            }),
            active_connection: self
                .model
                .active_connection()
                .map(|connection| connection.id.clone()),
            active_tab,
            tabs,
            split: self.model.split(),
            tab_panes,
            schema_visibility: self.schema_visibility.clone(),
            preferences: Some(self.preferences.clone()),
            sidebar_layout: self.sidebar_layout.clone(),
            filter_presets: self.table_filter_presets.clone(),
            table_layouts,
        }
    }

    pub(super) fn grid_layout(&self, tab_id: u64, cx: &Context<Self>) -> Option<GridLayout> {
        self.grids
            .get(&tab_id)
            .map(|grid| grid.read(cx).layout())
            .or_else(|| self.grid_layouts.get(&tab_id).cloned())
    }
}

fn session_path() -> Option<PathBuf> {
    Some(cellar_runtime::cellar_dir()?.join(SESSION_FILE))
}

fn save_to(path: &Path, contents: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary)?;
    file.write_all(contents)?;
    file.sync_all()?;
    drop(file);
    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs};

    use super::{save_to, SavedBounds, SessionState, SESSION_VERSION};

    #[test]
    fn session_json_is_versioned_and_rejects_invalid_window_sizes() {
        let state = SessionState {
            version: SESSION_VERSION,
            window: SavedBounds {
                x: 10.,
                y: 20.,
                width: 1200.,
                height: 800.,
            },
            sidebar_width: 252.,
            layout: None,
            active_connection: Some("one".into()),
            active_tab: None,
            tabs: Vec::new(),
            split: None,
            tab_panes: Vec::new(),
            schema_visibility: HashMap::new(),
            preferences: None,
            sidebar_layout: Vec::new(),
            filter_presets: HashMap::new(),
            table_layouts: HashMap::new(),
        };
        let json = serde_json::to_vec(&state).unwrap();
        let restored: SessionState = serde_json::from_slice(&json).unwrap();
        assert!(restored.window.valid());
        assert_eq!(restored.active_connection.as_deref(), Some("one"));
        assert!(!SavedBounds {
            width: f32::NAN,
            ..restored.window
        }
        .valid());
    }

    #[test]
    fn session_file_is_replaced_without_leaving_plaintext_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("session.json");
        save_to(&path, b"first").unwrap();
        save_to(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
