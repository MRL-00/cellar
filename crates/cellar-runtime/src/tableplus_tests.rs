use super::*;

/// Minimal stand-in for a real `Connections.plist`: the top level is an
/// array of dicts and each dict carries only the keys under test.
const FIXTURE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<array>
  <dict>
    <key>ID</key><string>11111111-1111-1111-1111-111111111111</string>
    <key>ConnectionName</key><string>Prod PG</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>db.example.com</string>
    <key>DatabasePort</key><string>6543</string>
    <key>DatabaseName</key><string>shop</string>
    <key>DatabaseUser</key><string>app</string>
    <key>Enviroment</key><string>production</string>
    <key>statusColor</key><string>#6D0000</string>
    <key>GroupID</key><string>g-1</string>
    <key>isOverSSH</key><true/>
    <key>tLSMode</key><integer>2</integer>
  </dict>
  <dict>
    <key>ID</key><string>22222222-2222-2222-2222-222222222222</string>
    <key>ConnectionName</key><string>Local MySQL</string>
    <key>Driver</key><string>MySQL</string>
    <key>DatabaseHost</key><string></string>
    <key>DatabasePort</key><string></string>
    <key>DatabaseName</key><string></string>
    <key>DatabaseUser</key><string>root</string>
    <key>statusColor</key><string>blue</string>
    <key>GroupID</key><string>g-missing</string>
  </dict>
  <dict>
    <key>ID</key><string>33333333-3333-3333-3333-333333333333</string>
    <key>ConnectionName</key><string>Notes</string>
    <key>Driver</key><string>SQLite</string>
    <key>DatabasePath</key><string>/Users/me/notes.db</string>
  </dict>
  <dict>
    <key>ID</key><string>44444444-4444-4444-4444-444444444444</string>
    <key>ConnectionName</key><string>Azure Sales</string>
    <key>Driver</key><string>Microsoft SQL Server</string>
    <key>DatabaseHost</key><string>epicprod.database.windows.net</string>
    <key>DatabasePort</key><string>1433</string>
    <key>DatabaseName</key><string>Sales</string>
    <key>DatabaseUser</key><string>dbadmin</string>
    <key>GroupID</key><string>g-2</string>
  </dict>
  <dict>
    <key>ID</key><string>55555555-5555-5555-5555-555555555555</string>
    <key>ConnectionName</key><string>Cache</string>
    <key>Driver</key><string>Redis</string>
    <key>DatabaseHost</key><string>redis.example.com</string>
  </dict>
  <dict>
    <key>ID</key><string>66666666-6666-6666-6666-666666666666</string>
    <key>ConnectionName</key><string></string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>anon.example.com</string>
    <key>DatabaseName</key><string>reports</string>
  </dict>
</array>
</plist>"#;

/// Stand-in for `ConnectionGroups.plist`: a top-level group, a group nested
/// under it, and a nameless row that must not become a folder.
const GROUPS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<array>
  <dict>
    <key>ID</key><string>g-1</string>
    <key>Name</key><string>OCO</string>
    <key>GroupID</key><string></string>
    <key>IsExpaned</key><true/>
  </dict>
  <dict>
    <key>ID</key><string>g-2</string>
    <key>Name</key><string>Reporting</string>
    <key>GroupID</key><string>g-1</string>
    <key>IsExpaned</key><false/>
  </dict>
  <dict>
    <key>ID</key><string>g-3</string>
    <key>Name</key><string></string>
    <key>GroupID</key><string></string>
  </dict>
</array>
</plist>"#;

fn groups() -> HashMap<String, String> {
    parse_groups(GROUPS.as_bytes())
}

fn by_name<'a>(r: &'a ConnectionImport, name: &str) -> &'a ConnectionConfig {
    r.connections
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no connection named {name}"))
}

#[test]
fn maps_supported_drivers_and_skips_the_rest() {
    let r = parse_connections(FIXTURE.as_bytes(), &groups());
    assert_eq!(r.connections.len(), 5, "5 supported rows imported");
    assert_eq!(r.skipped.len(), 1, "redis skipped");
    assert!(r.skipped[0].contains("Cache"));
    assert!(r.skipped[0].contains("unsupported engine: Redis"));
}

#[test]
fn postgres_row_maps_every_field() {
    let r = parse_connections(FIXTURE.as_bytes(), &groups());
    let pg = by_name(&r, "Prod PG");
    assert_eq!(pg.engine, Engine::Postgres);
    assert_eq!(pg.host, "db.example.com");
    assert_eq!(pg.port, 6543);
    assert_eq!(pg.database, "shop");
    assert_eq!(pg.user, "app");
    assert_eq!(pg.id, "prod-pg");
    assert_eq!(pg.env_tag, Some(EnvTag::Prod));
    assert_eq!(pg.color.as_deref(), Some("#6D0000"));
    assert_eq!(
        pg.ssl_mode,
        SslMode::Require,
        "tLSMode 2 is REQUIRED for PostgreSQL"
    );
    assert_eq!(pg.application_name.as_deref(), Some("cellar"));
}

#[test]
fn empty_host_port_and_database_fall_back_to_defaults() {
    let r = parse_connections(FIXTURE.as_bytes(), &groups());
    let my = by_name(&r, "Local MySQL");
    assert_eq!(my.engine, Engine::MySql);
    assert_eq!(my.host, "localhost");
    assert_eq!(my.port, 3306);
    assert_eq!(my.database, "mysql");
    assert_eq!(my.color, None, "non-#RRGGBB status color dropped");
    assert_eq!(my.env_tag, None);
}

#[test]
fn sqlite_uses_the_file_path_as_database() {
    let r = parse_connections(FIXTURE.as_bytes(), &groups());
    let db = by_name(&r, "Notes");
    assert_eq!(db.engine, Engine::Sqlite);
    assert_eq!(db.host, "");
    assert_eq!(db.port, 0);
    assert_eq!(db.database, "/Users/me/notes.db");
}

#[test]
fn azure_host_routes_to_the_azure_engine() {
    let r = parse_connections(FIXTURE.as_bytes(), &groups());
    let az = by_name(&r, "Azure Sales");
    assert_eq!(az.engine, Engine::Azure);
    assert_eq!(az.database, "Sales");
    assert_eq!(az.port, 1433);
}

#[test]
fn empty_name_falls_back_to_host_slash_database() {
    let r = parse_connections(FIXTURE.as_bytes(), &groups());
    let c = by_name(&r, "anon.example.com/reports");
    assert_eq!(c.id, "anon-example-com-reports");
}

#[test]
fn sqlite_without_any_path_is_skipped() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array><dict>
  <key>ConnectionName</key><string>Broken</string>
  <key>Driver</key><string>SQLite</string>
</dict></array></plist>"#;
    let r = parse_connections(xml.as_bytes(), &HashMap::new());
    assert!(r.connections.is_empty());
    assert!(r.skipped[0].contains("Broken"));
}

#[test]
fn rows_sharing_a_tableplus_id_collapse_to_one() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict>
    <key>ID</key><string>same-uuid</string>
    <key>ConnectionName</key><string>Dupe</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>h1</string>
  </dict>
  <dict>
    <key>ID</key><string>same-uuid</string>
    <key>ConnectionName</key><string>Dupe</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>h1</string>
  </dict>
</array></plist>"#;
    let r = merge(vec![parse_file(xml.as_bytes(), &HashMap::new())]);
    assert_eq!(
        r.connections.len(),
        1,
        "identical TablePlus IDs collapse silently"
    );
    assert!(r.skipped.is_empty());
}

#[test]
fn different_ids_with_the_same_name_report_a_duplicate_slug() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict>
    <key>ID</key><string>a</string>
    <key>ConnectionName</key><string>Shared Name</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>h1</string>
  </dict>
  <dict>
    <key>ID</key><string>b</string>
    <key>ConnectionName</key><string>Shared Name</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabaseHost</key><string>h2</string>
  </dict>
</array></plist>"#;
    let r = merge(vec![parse_file(xml.as_bytes(), &HashMap::new())]);
    assert_eq!(r.connections.len(), 1);
    assert_eq!(r.skipped.len(), 1);
    assert!(r.skipped[0].contains("duplicate id 'shared-name'"));
}

#[test]
fn same_connection_in_two_installs_is_imported_once() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array><dict>
  <key>ID</key><string>uuid-1</string>
  <key>ConnectionName</key><string>Shared</string>
  <key>Driver</key><string>PostgreSQL</string>
</dict></array></plist>"#;
    let r = merge(vec![
        parse_file(xml.as_bytes(), &HashMap::new()),
        parse_file(xml.as_bytes(), &HashMap::new()),
    ]);
    assert_eq!(r.connections.len(), 1);
    assert!(r.skipped.is_empty());
}

#[test]
fn a_row_with_an_unexpected_type_is_skipped_alone() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict>
    <key>ConnectionName</key><string>Odd</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>DatabasePort</key><integer>5432</integer>
  </dict>
  <dict>
    <key>ConnectionName</key><string>Fine</string>
    <key>Driver</key><string>PostgreSQL</string>
  </dict>
</array></plist>"#;
    let r = parse_connections(xml.as_bytes(), &HashMap::new());
    assert_eq!(r.connections.len(), 1);
    assert_eq!(r.connections[0].name, "Fine");
    assert_eq!(r.skipped.len(), 1);
    assert!(r.skipped[0].contains("entry #1"));
}

#[test]
fn tableplus_default_swatches_are_not_imported_as_accents() {
    assert_eq!(color("#686B6F"), None);
    assert_eq!(color("#f8f8f8"), None);
    assert_eq!(color("#007F3D").as_deref(), Some("#007F3D"));
    assert_eq!(color("blue"), None);
}

#[test]
fn a_grouped_connection_records_its_folder_name() {
    let r = parse_connections(FIXTURE.as_bytes(), &groups());
    assert_eq!(r.groups.get("prod-pg").map(String::as_str), Some("OCO"));
}

#[test]
fn a_nested_group_maps_to_its_own_leaf_name() {
    let r = parse_connections(FIXTURE.as_bytes(), &groups());
    assert_eq!(
        r.groups.get("azure-sales").map(String::as_str),
        Some("Reporting"),
        "Cellar folders are one level deep, so the leaf name wins"
    );
}

#[test]
fn an_ungrouped_connection_has_no_folder() {
    let r = parse_connections(FIXTURE.as_bytes(), &groups());
    assert!(by_name(&r, "Notes").id == "notes");
    assert!(!r.groups.contains_key("notes"));
}

#[test]
fn an_unknown_group_id_is_ignored() {
    let r = parse_connections(FIXTURE.as_bytes(), &groups());
    assert!(
        !r.groups.contains_key("local-mysql"),
        "a GroupID with no matching group is dropped, not invented"
    );
    assert_eq!(r.connections.len(), 5, "the connection still imports");
}

#[test]
fn a_nameless_group_is_not_a_folder() {
    assert!(!groups().contains_key("g-3"));
    assert_eq!(groups().len(), 2);
}

#[test]
fn a_garbage_groups_file_yields_no_groups_but_keeps_connections() {
    let bad = parse_groups(b"not a plist at all \x00\xff");
    assert!(bad.is_empty());
    let r = parse_connections(FIXTURE.as_bytes(), &bad);
    assert_eq!(r.connections.len(), 5);
    assert!(r.groups.is_empty());
}

#[test]
fn a_duplicate_connection_leaves_no_stray_group_entry() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array><dict>
  <key>ID</key><string>uuid-1</string>
  <key>ConnectionName</key><string>Shared</string>
  <key>Driver</key><string>PostgreSQL</string>
  <key>GroupID</key><string>g-1</string>
</dict></array></plist>"#;
    let r = merge(vec![
        parse_file(xml.as_bytes(), &groups()),
        parse_file(xml.as_bytes(), &groups()),
    ]);
    assert_eq!(r.connections.len(), 1);
    assert_eq!(r.groups.len(), 1, "one connection, one folder entry");
    assert_eq!(r.groups.get("shared").map(String::as_str), Some("OCO"));
}

#[test]
fn a_duplicate_name_keeps_the_first_rows_folder_not_the_dropped_ones() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict>
    <key>ID</key><string>a</string>
    <key>ConnectionName</key><string>Twin</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>GroupID</key><string>g-1</string>
  </dict>
  <dict>
    <key>ID</key><string>b</string>
    <key>ConnectionName</key><string>Twin</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>GroupID</key><string>g-2</string>
  </dict>
</array></plist>"#;
    let r = parse_connections(xml.as_bytes(), &groups());
    assert_eq!(r.connections.len(), 1);
    assert_eq!(r.groups.get("twin").map(String::as_str), Some("OCO"));
}

#[test]
fn a_dropped_duplicates_folder_never_leaks_onto_the_kept_row() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><array>
  <dict>
    <key>ID</key><string>a</string>
    <key>ConnectionName</key><string>Twin</string>
    <key>Driver</key><string>PostgreSQL</string>
  </dict>
  <dict>
    <key>ID</key><string>b</string>
    <key>ConnectionName</key><string>Twin</string>
    <key>Driver</key><string>PostgreSQL</string>
    <key>GroupID</key><string>g-1</string>
  </dict>
</array></plist>"#;
    let r = parse_connections(xml.as_bytes(), &groups());
    assert_eq!(r.connections.len(), 1);
    assert!(r.groups.is_empty(), "the kept row was ungrouped");
}

#[test]
fn tls_mode_maps_per_driver_and_falls_back_to_prefer() {
    assert_eq!(ssl_mode("PostgreSQL", 0), SslMode::Prefer);
    assert_eq!(ssl_mode("PostgreSQL", 1), SslMode::Disable);
    assert_eq!(
        ssl_mode("PostgreSQL", 3),
        SslMode::Prefer,
        "ALLOW has no Cellar equivalent"
    );
    assert_eq!(ssl_mode("PostgreSQL", 5), SslMode::VerifyFull);
    assert_eq!(ssl_mode("MySQL", 1), SslMode::Disable);
    assert_eq!(ssl_mode("MySQL", 4), SslMode::VerifyFull);
    assert_eq!(ssl_mode("MariaDB", 1), SslMode::Require);
    assert_eq!(ssl_mode("MariaDB", 2), SslMode::VerifyFull);
    assert_eq!(
        ssl_mode("Microsoft SQL Server", 2),
        SslMode::Prefer,
        "no dropdown for MSSQL"
    );
    assert_eq!(ssl_mode("PostgreSQL", 42), SslMode::Prefer, "unknown index");
}

#[test]
fn garbage_bytes_yield_an_empty_result() {
    let r = parse_connections(b"not a plist at all \x00\xff", &HashMap::new());
    assert!(r.connections.is_empty());
    assert_eq!(
        r.skipped.len(),
        1,
        "unparseable file is reported, not hidden"
    );
}
