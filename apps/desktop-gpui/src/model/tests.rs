use cellar_core::driver::{ConnectionConfig, Engine, SslMode};

use super::{
    AppModel, ConnectionState, QueryTarget, SchemaCompareConfig, SchemaCompareSource,
    SplitOrientation, TableLoadState, TableTarget,
};

fn config(id: &str) -> ConnectionConfig {
    ConnectionConfig {
        id: id.into(),
        name: id.into(),
        engine: Engine::Postgres,
        host: "localhost".into(),
        port: 5432,
        user: "cellar".into(),
        database: "cellar".into(),
        ssl_mode: SslMode::Prefer,
        env_tag: None,
        application_name: None,
        color: None,
    }
}

#[test]
fn selects_only_known_connections() {
    let mut model = AppModel::new(vec![config("one"), config("two")]);
    assert_eq!(
        model.active_connection().map(|config| config.id.as_str()),
        Some("one")
    );
    assert!(model.select_connection("two"));
    assert_eq!(
        model.active_connection().map(|config| config.id.as_str()),
        Some("two")
    );
    assert!(!model.select_connection("missing"));
    assert_eq!(
        model.active_connection().map(|config| config.id.as_str()),
        Some("two")
    );

    assert!(model.begin_connect("two"));
    assert_eq!(model.connection_state("two"), &ConnectionState::Connecting);
    assert!(!model.begin_connect("two"));
    model.finish_connect("two", Ok(Vec::new()));
    assert_eq!(model.connection_state("two"), &ConnectionState::Connected);
    assert!(!model.begin_connect("two"));
    assert!(model.begin_disconnect("two"));
    assert_eq!(
        model.connection_state("two"),
        &ConnectionState::Disconnecting
    );
    assert!(!model.begin_connect("two"));
    model.finish_disconnect("two");
    assert!(model.begin_connect("two"));
    model.finish_connect("two", Ok(Vec::new()));

    let target = TableTarget {
        connection_id: "two".into(),
        database: "cellar".into(),
        schema: "public".into(),
        table: "users".into(),
    };
    let (tab_id, load) = model.open_table(target.clone());
    assert!(load);
    assert_eq!(model.active_tab().map(|tab| tab.id), Some(tab_id));
    assert_eq!(model.open_table(target), (tab_id, false));
    model.close_tab(tab_id);
    assert!(model.active_tab().is_none());

    let query_id = model.new_query(QueryTarget {
        connection_id: "two".into(),
        database: "cellar".into(),
    });
    assert_eq!(model.active_tab().unwrap().title, "untitled-1.sql");
    model.close_tab(query_id);
    assert!(model.active_tab().is_none());
    let query_id = model.new_query(QueryTarget {
        connection_id: "two".into(),
        database: "cellar".into(),
    });
    assert_eq!(model.active_tab().unwrap().title, "untitled-2.sql");
    let second_query = model.new_query(QueryTarget {
        connection_id: "two".into(),
        database: "cellar".into(),
    });
    assert_eq!(model.active_tab().unwrap().title, "untitled-3.sql");
    assert!(model.move_tab(second_query, -1));
    assert_eq!(model.tabs()[0].id, second_query);
    model.close_tab(second_query);
    assert!(model.select_tab(query_id));
    assert!(model.begin_query(query_id));
    assert!(!model.begin_query(query_id));
    model.receive_query_page(query_id, 25);
    model.finish_query(query_id, Ok((25, 8)));

    assert!(model.begin_reconnect("two"));
    model.finish_disconnect("two");
    assert_eq!(
        model.connection_state("two"),
        &ConnectionState::Disconnected
    );
    assert!(model.databases("two").is_empty());

    model.upsert_connection(config("three"));
    model.remove_connection("two");
    assert!(model.connections().iter().all(|config| config.id != "two"));
    assert!(model.tabs().is_empty());
}

#[test]
fn deleting_either_live_comparison_connection_closes_the_tab() {
    let mut model = AppModel::new(vec![config("source"), config("target")]);
    model.open_schema_compare(SchemaCompareConfig {
        source: SchemaCompareSource::Snapshot {
            id: "snapshot".into(),
            schema: "public".into(),
            label: None,
        },
        target: SchemaCompareSource::Live {
            connection_id: "target".into(),
            database: "cellar".into(),
            schema: "public".into(),
            label: None,
        },
    });

    model.remove_connection("target");

    assert!(model.tabs().is_empty());
}

#[test]
fn stale_table_loads_cannot_replace_newer_results() {
    let mut model = AppModel::new(vec![config("one")]);
    let target = TableTarget {
        connection_id: "one".into(),
        database: "cellar".into(),
        schema: "public".into(),
        table: "users".into(),
    };
    let (tab_id, _) = model.open_table(target);
    let stale = model.next_table_load(tab_id);
    let current = model.next_table_load(tab_id);

    assert!(!model.finish_table_load(tab_id, stale, Err("stale".into())));
    assert!(matches!(
        &model.active_tab().unwrap().kind,
        super::TabKind::Table {
            state: TableLoadState::Loading,
            ..
        }
    ));
    assert!(model.finish_table_load(tab_id, current, Ok((3, Some(3)))));
    assert!(matches!(
        &model.active_tab().unwrap().kind,
        super::TabKind::Table {
            state: TableLoadState::Loaded,
            ..
        }
    ));
}

#[test]
fn split_tracks_focus_and_collapses_when_a_pane_empties() {
    let mut model = AppModel::new(vec![config("one")]);
    let first = model.new_query(QueryTarget {
        connection_id: "one".into(),
        database: "cellar".into(),
    });
    let second = model.new_query(QueryTarget {
        connection_id: "one".into(),
        database: "cellar".into(),
    });

    assert!(model.toggle_split(SplitOrientation::Vertical));
    assert_eq!(model.tab_pane(first), 0);
    assert_eq!(model.tab_pane(second), 1);
    assert_eq!(model.focused_pane(), 1);
    assert!(model.select_tab(first));
    assert_eq!(model.focused_pane(), 0);
    assert_eq!(model.active_tab_in_pane(1).map(|tab| tab.id), Some(second));

    model.close_tab(second);
    assert_eq!(model.split(), None);
    assert_eq!(model.active_tab().map(|tab| tab.id), Some(first));
}

#[test]
fn dragged_tabs_reorder_move_and_redirect_splits() {
    let mut model = AppModel::new(vec![config("one")]);
    let target = QueryTarget {
        connection_id: "one".into(),
        database: "cellar".into(),
    };
    let first = model.new_query(target.clone());
    let second = model.new_query(target.clone());
    let third = model.new_query(target);

    assert!(model.toggle_split(SplitOrientation::Vertical));
    assert!(model.reorder_tab(first, third));
    assert_eq!(model.tab_pane(first), 1);
    assert_eq!(model.tabs()[1].id, first);
    assert!(model.move_tab_to_pane(third, 0));
    assert_eq!(model.tab_pane(third), 0);
    assert!(model.drop_tab_to_split(second, SplitOrientation::Horizontal, 1));
    assert_eq!(model.split(), Some(SplitOrientation::Horizontal));
    assert_eq!(model.tab_pane(second), 1);
    assert_eq!(model.focused_pane(), 1);
}

#[test]
fn query_pages_are_visible_before_completion() {
    let mut model = AppModel::new(vec![config("one")]);
    let tab_id = model.new_query(QueryTarget {
        connection_id: "one".into(),
        database: "cellar".into(),
    });
    assert!(model.begin_query(tab_id));
    model.receive_query_page(tab_id, 250);
    assert!(matches!(
        &model.active_tab().unwrap().kind,
        super::TabKind::Query {
            state: super::QueryState::Running { rows_received: 250 },
            ..
        }
    ));
    model.finish_query(tab_id, Ok((250, 12)));
    assert!(matches!(
        &model.active_tab().unwrap().kind,
        super::TabKind::Query {
            state: super::QueryState::Complete { .. },
            ..
        }
    ));
    assert!(model.set_query_database(tab_id, "analytics".into()));
    assert!(matches!(
        &model.active_tab().unwrap().kind,
        super::TabKind::Query { target, state: super::QueryState::Editing }
            if target.database == "analytics"
    ));
    assert!(!model.set_query_database(tab_id, "analytics".into()));
}
