mod ai;
mod ai_auth;
mod ai_context;
mod ai_history;
mod ai_message;
mod ai_panel;
mod bottom_panel_header;
mod bottom_panel_support;
mod bottom_panel_views;
mod command_palette;
mod commit;
mod confirm;
mod connection_editor;
mod connection_editor_support;
mod connection_editor_view;
mod connection_import;
mod connections;
mod context_menu;
mod empty_state;
mod er_diagram;
mod er_layout;
mod find_usages;
mod history_workspace;
mod import_workspace;
mod panel_resize;
mod plan_panel;
pub(crate) mod preferences;
mod query_control;
mod query_editor;
mod query_parameter_view;
mod query_params;
mod query_plans;
mod query_templates;
mod query_widgets;
mod query_workspace;
mod render;
mod schema_compare;
mod schema_compare_dialog;
mod schema_compare_support;
mod schema_tree;
mod schema_visibility;
mod session;
mod settings;
mod settings_ai;
mod settings_data;
mod settings_search;
mod settings_system;
mod settings_workspace;
mod setup_transfer;
mod setup_transfer_view;
mod setup_transfer_widgets;
mod shell;
mod shell_bottom_export;
mod shell_widgets;
mod sidebar_drag;
pub(crate) mod sidebar_layout;
mod sidebar_menu;
mod sql_completion;
mod status_bar;
mod tab_context_menu;
mod table_filter_bar;
mod table_presets;
mod table_quick_filter;
mod table_workspace;
mod updater;
mod workspace;

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use cellar_core::{
    driver::{ConnectionConfig, EnvTag},
    query::{QueryPlan, QueryResultSummary, TableFilterClause, TableSortClause},
};
use cellar_runtime::history::{HistoryStore, QueryHistoryRecord};
use cellar_runtime::ConnectionRegistry;
use gpui::{
    div, prelude::*, px, Bounds, Context, Entity, FocusHandle, KeyDownEvent, MouseButton,
    MouseDownEvent, Pixels, Render, SharedString, Subscription, Window,
};
use gpui_component::{
    input::Input,
    input::InputState,
    slider::{SliderEvent, SliderState, SliderValue},
    Icon,
};

use ai::AiState;
use commit::CommitReview;
use connection_editor::ConnectionEditor;
use connection_import::ConnectionImport;
use import_workspace::DataImport;
use query_params::QueryParameterInput;
use query_templates::SaveTemplateEditor;
pub(crate) use session::SessionState;
use sidebar_layout::{SidebarItem, SidebarLayout};

use cellar_desktop_gpui::{
    grid::{DataGrid, GridLayout},
    model::{AppModel, ConnectionState},
    theme::{
        ui_px, BG, BORDER, BORDER_SEPARATOR, FG, FG_DISABLED, FG_MUTED, INSERT, INSET, PANEL_MUTED,
        PANEL_RAISED, WARN, WARN_SOFT,
    },
};

pub struct CellarApp {
    model: AppModel,
    registry: Arc<ConnectionRegistry>,
    runtime: Arc<tokio::runtime::Runtime>,
    driver_infos: HashMap<String, cellar_core::driver::DriverInfo>,
    grids: HashMap<u64, Entity<DataGrid>>,
    grid_layouts: HashMap<u64, GridLayout>,
    table_layouts: HashMap<String, GridLayout>,
    editors: HashMap<u64, Entity<InputState>>,
    query_editor_subscriptions: HashMap<u64, Subscription>,
    query_saved_sql: HashMap<u64, String>,
    query_params: HashMap<u64, Vec<QueryParameterInput>>,
    query_wrap: HashMap<u64, bool>,
    preferences: preferences::Preferences,
    table_sorts: HashMap<u64, TableSortClause>,
    table_filters: HashMap<u64, Vec<TableFilterClause>>,
    table_filter_operators: HashMap<u64, cellar_core::query::TableFilterOperator>,
    table_filter_inputs: HashMap<u64, Entity<InputState>>,
    table_filter_columns: HashMap<u64, usize>,
    table_filter_composers: HashSet<u64>,
    table_quick_filter_inputs: HashMap<u64, Entity<InputState>>,
    table_quick_filter_columns: HashMap<u64, usize>,
    table_quick_filters: HashMap<u64, String>,
    table_quick_filter_subscriptions: HashMap<u64, Subscription>,
    table_filter_presets: HashMap<String, Vec<table_presets::FilterPreset>>,
    table_preset_menu: Option<table_presets::PresetMenu>,
    table_preset_draft: Option<table_presets::PresetDraft>,
    table_preset_subscription: Option<Subscription>,
    query_summaries: HashMap<u64, QueryResultSummary>,
    last_query_metrics: Option<(u64, bool, u64)>,
    query_generations: HashMap<u64, u64>,
    query_confirmations: HashMap<u64, (String, u8)>,
    query_plans: HashMap<u64, Result<QueryPlan, String>>,
    plan_modes: HashMap<u64, cellar_core::query::PlanMode>,
    plan_loading: HashSet<u64>,
    er_views: HashMap<u64, er_layout::ErViewState>,
    schema_compares: HashMap<u64, schema_compare::SchemaCompareWorkspace>,
    schema_compare_dialog: Option<schema_compare_dialog::SchemaCompareDialog>,
    analyze_confirmations: HashMap<u64, (String, u8)>,
    history: Option<Arc<HistoryStore>>,
    history_records: Vec<QueryHistoryRecord>,
    history_loading: bool,
    history_error: Option<String>,
    history_generation: u64,
    bottom_message_filter: bottom_panel_views::MessageFilter,
    bottom_retain_notice_tabs: HashSet<u64>,
    bottom_history_search: Entity<InputState>,
    _bottom_history_search_subscription: Subscription,
    sidebar_filter: Entity<InputState>,
    _sidebar_filter_subscription: Subscription,
    command_palette: Option<Entity<InputState>>,
    command_palette_subscription: Option<Subscription>,
    command_palette_active: usize,
    query_templates: Vec<cellar_runtime::query_templates::QueryTemplate>,
    save_template_editor: Option<SaveTemplateEditor>,
    confirmation: Option<confirm::Confirmation>,
    confirmation_focus: FocusHandle,
    pending_connection_errors: Vec<String>,
    settings_open: bool,
    settings_category: settings::SettingsCategory,
    settings_search: Entity<InputState>,
    _settings_search_subscription: Subscription,
    font_size_input: Entity<InputState>,
    _font_size_input_subscription: Subscription,
    font_size_slider: Entity<SliderState>,
    _font_size_subscription: Subscription,
    connection_menu: Option<context_menu::ConnectionMenu>,
    table_menu: Option<context_menu::TableMenu>,
    schema_menu: Option<context_menu::SchemaMenu>,
    tab_menu: Option<context_menu::TabMenu>,
    find_usages: Option<find_usages::FindUsagesState>,
    find_usages_generation: u64,
    schema_visibility: HashMap<String, schema_visibility::SchemaVisibilityPrefs>,
    schema_visibility_editor: Option<schema_visibility::SchemaVisibilityEditor>,
    closed_tabs: Vec<workspace::ClosedTab>,
    sidebar_open: bool,
    show_empty_state: bool,
    bottom_panel_open: bool,
    bottom_export_menu: bool,
    bottom_panel_tab: shell::BottomPanelTab,
    right_panel_open: bool,
    right_panel_width: f32,
    bottom_panel_height: f32,
    tauri_layout_loaded: bool,
    connection_editor: Option<ConnectionEditor>,
    connection_import: Option<ConnectionImport>,
    commit_review: Option<CommitReview>,
    data_import: Option<DataImport>,
    sidebar_width: f32,
    sidebar_resize: Option<(f32, f32)>,
    right_panel_resize: Option<(f32, f32)>,
    bottom_panel_resize: Option<(f32, f32)>,
    window_bounds: Bounds<Pixels>,
    sidebar_layout: Vec<SidebarItem>,
    sidebar_menu: Option<gpui::Point<Pixels>>,
    folder_menu: Option<sidebar_menu::FolderMenu>,
    folder_rename: Option<sidebar_menu::FolderRename>,
    folder_rename_subscription: Option<Subscription>,
    _appearance_subscription: Subscription,
    setup_transfer: Option<setup_transfer::SetupTransfer>,
    ai: AiState,
    ai_generation: u64,
    ai_task: Option<gpui::Task<()>>,
    ai_auth_poll: Option<gpui::Task<()>>,
    updater_status: updater::UpdateStatus,
    updater_last_checked: Option<String>,
    dismissed_update_version: Option<String>,
    updater_task: Option<gpui::Task<()>>,
    query_database_menu: Option<shell::QueryDatabaseMenu>,
    last_titlebar_press: Option<Instant>,
}

impl CellarApp {
    pub fn new(
        connections: Vec<ConnectionConfig>,
        registry: Arc<ConnectionRegistry>,
        runtime: Arc<tokio::runtime::Runtime>,
        window_bounds: Bounds<Pixels>,
        sidebar_layout: SidebarLayout,
        preferences: preferences::Preferences,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let shell = sidebar_layout.shell;
        let sidebar_filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter…"));
        let sidebar_filter_subscription = cx.observe(&sidebar_filter, |_, _, cx| cx.notify());
        let settings_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search settings…"));
        let settings_search_subscription = cx.observe(&settings_search, |_, _, cx| cx.notify());
        let font_size_input = cx.new(|cx| {
            InputState::new(window, cx).default_value(format!("{:.1}", preferences.font_size_px))
        });
        let font_size_input_subscription =
            cx.observe_in(&font_size_input, window, |this, input, window, cx| {
                let Some(value) = preferences::parse_font_size(input.read(cx).value().as_ref())
                else {
                    return;
                };
                if (this.preferences.font_size_px - value).abs() < f32::EPSILON {
                    return;
                }
                this.preferences.font_size_px = value;
                this.font_size_slider
                    .update(cx, |slider, cx| slider.set_value(value, window, cx));
                this.apply_appearance(window, cx);
            });
        let bottom_history_search =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search SQL or errors"));
        let bottom_history_search_subscription =
            cx.observe(&bottom_history_search, |this, _, cx| {
                this.refresh_history(cx);
                cx.notify();
            });
        let font_size_slider = cx.new(|_| {
            SliderState::new()
                .min(10.)
                .max(22.)
                .step(0.5)
                .default_value(preferences.font_size_px)
        });
        let font_size_subscription = cx.subscribe_in(
            &font_size_slider,
            window,
            |this, _, event: &SliderEvent, window, cx| {
                let SliderEvent::Change(SliderValue::Single(value)) = event else {
                    return;
                };
                this.preferences.font_size_px = *value;
                let text = format!("{value:.1}");
                if this.font_size_input.read(cx).value() != text {
                    this.font_size_input
                        .update(cx, |input, cx| input.set_value(text, window, cx));
                }
                this.apply_appearance(window, cx);
            },
        );
        let appearance_subscription = cx.observe_window_appearance(window, |this, window, cx| {
            this.apply_appearance(window, cx);
        });
        let ai = AiState::new(window, cx);
        Self {
            model: AppModel::new(connections),
            registry,
            runtime,
            driver_infos: HashMap::new(),
            grids: HashMap::new(),
            grid_layouts: HashMap::new(),
            table_layouts: HashMap::new(),
            editors: HashMap::new(),
            query_editor_subscriptions: HashMap::new(),
            query_saved_sql: HashMap::new(),
            query_params: HashMap::new(),
            query_wrap: HashMap::new(),
            preferences,
            table_sorts: HashMap::new(),
            table_filters: HashMap::new(),
            table_filter_operators: HashMap::new(),
            table_filter_inputs: HashMap::new(),
            table_filter_columns: HashMap::new(),
            table_filter_composers: HashSet::new(),
            table_quick_filter_inputs: HashMap::new(),
            table_quick_filter_columns: HashMap::new(),
            table_quick_filters: HashMap::new(),
            table_quick_filter_subscriptions: HashMap::new(),
            table_filter_presets: HashMap::new(),
            table_preset_menu: None,
            table_preset_draft: None,
            table_preset_subscription: None,
            query_summaries: HashMap::new(),
            last_query_metrics: None,
            query_generations: HashMap::new(),
            query_confirmations: HashMap::new(),
            query_plans: HashMap::new(),
            plan_modes: HashMap::new(),
            plan_loading: HashSet::new(),
            er_views: HashMap::new(),
            schema_compares: HashMap::new(),
            schema_compare_dialog: None,
            analyze_confirmations: HashMap::new(),
            history: None,
            history_records: Vec::new(),
            history_loading: true,
            history_error: None,
            history_generation: 0,
            bottom_message_filter: bottom_panel_views::MessageFilter::All,
            bottom_retain_notice_tabs: HashSet::new(),
            bottom_history_search,
            _bottom_history_search_subscription: bottom_history_search_subscription,
            sidebar_filter,
            _sidebar_filter_subscription: sidebar_filter_subscription,
            command_palette: None,
            command_palette_subscription: None,
            command_palette_active: 0,
            query_templates: Vec::new(),
            save_template_editor: None,
            confirmation: None,
            confirmation_focus: cx.focus_handle().tab_stop(true),
            pending_connection_errors: Vec::new(),
            settings_open: false,
            settings_category: settings::SettingsCategory::Appearance,
            settings_search,
            _settings_search_subscription: settings_search_subscription,
            font_size_input,
            _font_size_input_subscription: font_size_input_subscription,
            font_size_slider,
            _font_size_subscription: font_size_subscription,
            connection_menu: None,
            table_menu: None,
            schema_menu: None,
            tab_menu: None,
            find_usages: None,
            find_usages_generation: 0,
            schema_visibility: HashMap::new(),
            schema_visibility_editor: None,
            closed_tabs: Vec::new(),
            sidebar_open: shell.is_none_or(|layout| layout.panels.left),
            show_empty_state: false,
            bottom_panel_open: shell.is_some_and(|layout| layout.panels.bottom),
            bottom_export_menu: false,
            bottom_panel_tab: shell::BottomPanelTab::Results,
            right_panel_open: shell.is_some_and(|layout| layout.panels.right),
            right_panel_width: shell.map_or(380., |layout| layout.right_width),
            bottom_panel_height: shell.map_or(280., |layout| layout.bottom_height),
            tauri_layout_loaded: shell.is_some(),
            connection_editor: None,
            connection_import: None,
            commit_review: None,
            data_import: None,
            sidebar_width: shell.map_or(256., |layout| layout.left_width),
            sidebar_resize: None,
            right_panel_resize: None,
            bottom_panel_resize: None,
            window_bounds,
            sidebar_layout: sidebar_layout.items,
            sidebar_menu: None,
            folder_menu: None,
            folder_rename: None,
            folder_rename_subscription: None,
            _appearance_subscription: appearance_subscription,
            setup_transfer: None,
            ai,
            ai_generation: 0,
            ai_task: None,
            ai_auth_poll: None,
            updater_status: updater::UpdateStatus::Idle,
            updater_last_checked: None,
            dismissed_update_version: None,
            updater_task: None,
            query_database_menu: None,
            last_titlebar_press: None,
        }
    }

    fn connection_row(
        &self,
        config: &ConnectionConfig,
        active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = config.id.clone();
        let menu_id = id.clone();
        let production = config.env_tag == Some(EnvTag::Prod);
        let state_color = match self.model.connection_state(&config.id) {
            ConnectionState::Disconnected => FG_DISABLED,
            ConnectionState::Connecting => WARN,
            ConnectionState::Disconnecting => WARN,
            ConnectionState::Connected => INSERT,
            ConnectionState::Error(_) => WARN,
        };
        div()
            .id(SharedString::from(config.id.clone()))
            .tab_index(0)
            .cursor_pointer()
            .h(ui_px(26.))
            .rounded(ui_px(3.))
            .flex()
            .items_center()
            .gap_1()
            .pl_1()
            .pr(ui_px(6.))
            .text_size(ui_px(14.))
            .text_color(FG)
            .bg(BG)
            .hover(|style| style.bg(PANEL_MUTED))
            .child(
                div()
                    .size(ui_px(14.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::empty()
                            .path(if active {
                                "icons/chevron-down.svg"
                            } else {
                                "icons/chevron-right.svg"
                            })
                            .size(ui_px(10.))
                            .text_color(FG_MUTED),
                    ),
            )
            .child(div().flex_1().truncate().child(config.name.clone()))
            .when(production, |element| {
                element.child(
                    div()
                        .rounded(ui_px(3.))
                        .bg(WARN_SOFT)
                        .px(ui_px(4.))
                        .text_size(ui_px(9.))
                        .text_color(WARN)
                        .child("PROD"),
                )
            })
            .child(
                div()
                    .ml_1()
                    .size(ui_px(6.))
                    .flex_shrink_0()
                    .rounded(ui_px(3.))
                    .bg(state_color),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.connection_menu = Some(context_menu::ConnectionMenu {
                        connection_id: menu_id.clone(),
                        position: event.position,
                        show_folders: false,
                    });
                    cx.notify();
                }),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.model.select_connection(&id);
                this.start_connect(id.clone(), window, cx);
            }))
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let active_id = self
            .model
            .active_connection()
            .map(|config| config.id.as_str());
        let sidebar_query = self.sidebar_filter.read(cx).value().trim().to_lowercase();
        let match_count = self
            .model
            .connections()
            .iter()
            .filter(|config| {
                sidebar_query.is_empty()
                    || sidebar_layout::connection_matches(config, &sidebar_query)
            })
            .count();
        div()
            .relative()
            .w(ui_px(self.sidebar_width))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(BG)
            .border_r_1()
            .border_color(BORDER)
            .child(
                div()
                    .h(ui_px(28.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .pl(ui_px(10.))
                    .pr(ui_px(8.))
                    .text_size(ui_px(12.))
                    .text_color(FG_MUTED)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(ui_px(6.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("CONNECTIONS")
                            .child(
                                div()
                                    .rounded(ui_px(8.))
                                    .bg(PANEL_RAISED)
                                    .px(ui_px(6.))
                                    .py(ui_px(1.))
                                    .text_size(ui_px(11.))
                                    .child(self.model.connections().len().to_string()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(ui_px(1.))
                            .child(
                                div()
                                    .id("new-connection")
                                    .cursor_pointer()
                                    .size(ui_px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(ui_px(4.))
                                    .text_color(FG_MUTED)
                                    .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                                    .child(Icon::empty().path("icons/plus.svg").size(ui_px(12.)))
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_connection_editor(None, window, cx);
                                    })),
                            )
                            .child(
                                div()
                                    .id("connection-actions")
                                    .cursor_pointer()
                                    .size(ui_px(22.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded(ui_px(4.))
                                    .text_color(FG_MUTED)
                                    .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                                    .child(
                                        Icon::empty().path("icons/ellipsis.svg").size(ui_px(12.)),
                                    )
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, event: &MouseDownEvent, _, cx| {
                                            this.sidebar_menu = Some(event.position);
                                            cx.notify();
                                        }),
                                    ),
                            ),
                    ),
            )
            .child(
                div()
                    .h(ui_px(28.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(ui_px(6.))
                    .mx(ui_px(8.))
                    .mb(ui_px(6.))
                    .px(ui_px(8.))
                    .rounded(ui_px(4.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(INSET)
                    .text_color(FG_MUTED)
                    .child(Icon::empty().path("icons/search.svg").size(ui_px(11.)))
                    .child(
                        Input::new(&self.sidebar_filter)
                            .h_full()
                            .flex_1()
                            .appearance(false),
                    )
                    .child(shell_widgets::keycap("⌘F")),
            )
            .child(
                div()
                    .id("connection-list")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(self.sidebar_rows(active_id, cx))
                    .pb(ui_px(12.))
                    .when(self.model.connections().is_empty(), |element| {
                        element
                            .child(
                                div()
                                    .id("empty-new-connection")
                                    .cursor_pointer()
                                    .h(ui_px(30.))
                                    .mx(ui_px(8.))
                                    .mt_1()
                                    .mb_3()
                                    .flex()
                                    .items_center()
                                    .gap(ui_px(6.))
                                    .rounded(ui_px(4.))
                                    .border_1()
                                    .border_dashed()
                                    .border_color(BORDER)
                                    .px_2()
                                    .text_color(cellar_desktop_gpui::theme::FG_SECONDARY)
                                    .hover(|style| {
                                        style
                                            .border_color(cellar_desktop_gpui::theme::ACCENT)
                                            .bg(cellar_desktop_gpui::theme::accent_soft())
                                            .text_color(cellar_desktop_gpui::theme::ACCENT)
                                    })
                                    .child(Icon::empty().path("icons/plus.svg").size(ui_px(11.)))
                                    .child("New connection")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_connection_editor(None, window, cx)
                                    })),
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_5()
                                    .text_center()
                                    .text_size(ui_px(12.))
                                    .text_color(FG_MUTED)
                                    .child("no connections yet"),
                            )
                    })
                    .when(
                        !self.model.connections().is_empty() && match_count == 0,
                        |element| {
                            element.child(
                                div()
                                    .px_3()
                                    .py_5()
                                    .text_center()
                                    .text_size(ui_px(12.))
                                    .text_color(FG_MUTED)
                                    .child("no matches"),
                            )
                        },
                    ),
            )
            .child(
                div()
                    .h(ui_px(39.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .px_2()
                    .py(ui_px(6.))
                    .border_t_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .id("open-settings")
                            .cursor_pointer()
                            .h(ui_px(27.))
                            .flex()
                            .items_center()
                            .gap(ui_px(6.))
                            .rounded(ui_px(4.))
                            .px(ui_px(6.))
                            .text_size(ui_px(14.))
                            .text_color(cellar_desktop_gpui::theme::FG_SECONDARY)
                            .child(Icon::empty().path("icons/settings.svg").size(ui_px(13.)))
                            .child("Settings")
                            .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.open_settings(settings::SettingsCategory::Appearance, cx);
                            })),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .right(ui_px(-3.))
                    .top_0()
                    .bottom_0()
                    .w(ui_px(7.))
                    .group("left-panel-resizer")
                    .cursor_col_resize()
                    .child(
                        div()
                            .absolute()
                            .left(ui_px(3.))
                            .top_0()
                            .bottom_0()
                            .w(ui_px(1.))
                            .bg(if self.sidebar_resize.is_some() {
                                cellar_desktop_gpui::theme::ACCENT.rgba()
                            } else {
                                BORDER_SEPARATOR.rgba()
                            })
                            .group_hover("left-panel-resizer", |style| {
                                style.bg(cellar_desktop_gpui::theme::accent(0.32))
                            }),
                    )
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _, cx| {
                            this.sidebar_resize =
                                Some((f32::from(event.position.x), this.sidebar_width));
                            cx.notify();
                        }),
                    ),
            )
    }
}
