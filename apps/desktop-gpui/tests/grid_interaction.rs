//! Grid interaction tests driven through real window events: selecting rows
//! with the mouse and copying with the platform clipboard shortcut.
use cellar_core::{
    query::{NoticeCapture, QueryResult},
    value::{CellValue, ColumnMeta},
};
use cellar_desktop_gpui::{grid::DataGrid, theme};
use gpui::{point, px, Modifiers, TestAppContext};
use gpui_component::input::Copy;

const HEADER_HEIGHT: f32 = 26.;

fn column(name: &str, data_type: &str) -> ColumnMeta {
    ColumnMeta {
        name: name.into(),
        data_type: data_type.into(),
        nullable: true,
    }
}

fn sample_result() -> QueryResult {
    QueryResult {
        columns: vec![
            column("mssite", "nvarchar(64)"),
            column("Email", "nvarchar(128)"),
            column("Phone", "nvarchar(32)"),
        ],
        rows: vec![
            vec![
                CellValue::Text("2f0c0fc6-9a7c-4f3b-9d10-6d0a1f3e8b21".into()),
                CellValue::Text("jack@epiczone.me".into()),
                CellValue::Null,
            ],
            vec![
                CellValue::Text("8dfada7c-6bf2-4d5e-9f13-22b7c9e0a441".into()),
                CellValue::Text("thegtand@gmail.co.nz".into()),
                CellValue::Text("+6495614222".into()),
            ],
        ],
        notices: Vec::new(),
        notice_capture: NoticeCapture::unsupported("test"),
        rows_affected: None,
        duration_ms: 0,
        truncated: false,
        total_rows: None,
    }
}

/// Row number gutter, first row: the click target users select rows with.
fn first_row_gutter() -> gpui::Point<gpui::Pixels> {
    let scale = theme::ui_scale();
    point(
        px(18. * scale),
        px(HEADER_HEIGHT * scale + theme::row_height() / 2.),
    )
}

fn secondary(keystroke: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("cmd-{keystroke}")
    } else {
        format!("ctrl-{keystroke}")
    }
}

/// The grid renders gpui-component icons and popovers, which need the
/// component theme in `App` state.
fn init_components(cx: &mut TestAppContext) {
    cx.update(|cx| gpui_component::init(cx));
}

fn clipboard_text(cx: &TestAppContext) -> String {
    cx.read_from_clipboard()
        .and_then(|item| item.text())
        .unwrap_or_default()
}

#[gpui::test]
fn selecting_a_row_and_pressing_the_copy_shortcut_writes_json(cx: &mut TestAppContext) {
    init_components(cx);
    let (_grid, cx) = cx.add_window_view(|_, cx| DataGrid::new(sample_result(), cx));
    cx.simulate_click(first_row_gutter(), Modifiers::default());
    cx.simulate_keystrokes(&secondary("c"));

    let text = clipboard_text(cx);
    let rows: serde_json::Value = serde_json::from_str(&text).expect("clipboard holds JSON");
    let rows = rows.as_array().expect("JSON array of rows");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["mssite"], "2f0c0fc6-9a7c-4f3b-9d10-6d0a1f3e8b21");
    assert_eq!(rows[0]["Email"], "jack@epiczone.me");
    assert!(rows[0]["Phone"].is_null());
}

#[gpui::test]
fn a_second_gutter_click_extends_the_json_selection(cx: &mut TestAppContext) {
    init_components(cx);
    let (_grid, cx) = cx.add_window_view(|_, cx| DataGrid::new(sample_result(), cx));
    let first = first_row_gutter();
    let second = point(first.x, first.y + px(theme::row_height()));
    cx.simulate_click(first, Modifiers::default());
    cx.simulate_click(second, Modifiers::default());
    cx.simulate_keystrokes(&secondary("c"));

    let rows: serde_json::Value =
        serde_json::from_str(&clipboard_text(cx)).expect("clipboard holds JSON");
    assert_eq!(rows.as_array().expect("JSON array of rows").len(), 1);

    // Shift-click extends the range, so both rows are copied.
    cx.simulate_click(first, Modifiers::default());
    cx.simulate_click(
        second,
        Modifiers {
            shift: true,
            ..Modifiers::default()
        },
    );
    cx.simulate_keystrokes(&secondary("c"));
    let rows: serde_json::Value =
        serde_json::from_str(&clipboard_text(cx)).expect("clipboard holds JSON");
    assert_eq!(rows.as_array().expect("JSON array of rows").len(), 2);
}

#[gpui::test]
fn the_edit_menu_copy_action_copies_the_selection(cx: &mut TestAppContext) {
    init_components(cx);
    let (_grid, cx) = cx.add_window_view(|_, cx| DataGrid::new(sample_result(), cx));
    cx.simulate_click(first_row_gutter(), Modifiers::default());
    cx.dispatch_action(Copy);

    let rows: serde_json::Value =
        serde_json::from_str(&clipboard_text(cx)).expect("clipboard holds JSON");
    assert_eq!(rows.as_array().expect("JSON array of rows").len(), 1);
}

#[gpui::test]
fn a_fresh_grid_fits_columns_to_their_values(cx: &mut TestAppContext) {
    init_components(cx);
    let (grid, cx) = cx.add_window_view(|_, cx| DataGrid::new(sample_result(), cx));
    let widths = cx.update(|_, cx| grid.read(cx).layout().portable().widths);

    // The uuid column is the widest value, so it is wider than the email
    // column, and both start from a fitted width rather than the old default.
    assert!(
        widths["mssite"] > widths["Email"],
        "expected the uuid column to be widest, got {widths:?}"
    );
    assert!(
        widths["Email"] > 150.,
        "expected the email column to fit its values, got {widths:?}"
    );
}

#[gpui::test]
fn a_clicked_cell_copies_tsv_with_a_header(cx: &mut TestAppContext) {
    init_components(cx);
    let (_grid, cx) = cx.add_window_view(|_, cx| DataGrid::new(sample_result(), cx));
    let row_y = first_row_gutter().y;
    cx.simulate_click(point(px(120.), row_y), Modifiers::default());
    cx.simulate_keystrokes(&secondary("c"));

    let text = clipboard_text(cx);
    assert_eq!(text, "mssite\r\n2f0c0fc6-9a7c-4f3b-9d10-6d0a1f3e8b21");
}
