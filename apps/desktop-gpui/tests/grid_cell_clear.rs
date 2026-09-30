//! Backspace/Delete in an editable table grid, driven through real window
//! events: clearing a cell must never turn into a row delete.
use std::{cell::RefCell, rc::Rc};

use cellar_core::{
    query::{NoticeCapture, QueryResult},
    schema::{Column, Table},
    value::{CellValue, ColumnMeta},
};
use cellar_desktop_gpui::{
    grid::{DataGrid, DataGridEvent},
    model::TableTarget,
    theme,
};
use cellar_diff::{RowChange, TableChangeRequest};
use gpui::{point, px, AppContext as _, Entity, Modifiers, TestAppContext, VisualTestContext};

const HEADER_HEIGHT: f32 = 26.;

fn column(name: &str, data_type: &str, key: bool, ordinal: u32) -> Column {
    Column {
        name: name.into(),
        data_type: data_type.into(),
        nullable: !key,
        default: None,
        is_primary_key: key,
        ordinal,
        comment: None,
    }
}

fn users_grid(cx: &mut gpui::Context<DataGrid>) -> DataGrid {
    let target = TableTarget {
        connection_id: "one".into(),
        database: "cellar".into(),
        schema: "public".into(),
        table: "users".into(),
    };
    let table = Table {
        name: "users".into(),
        schema: "public".into(),
        row_count: Some(1),
        columns: vec![
            column("id", "int8", true, 1),
            column("name", "text", false, 2),
        ],
        primary_key: vec!["id".into()],
        foreign_keys: Vec::new(),
        indexes: Vec::new(),
    };
    let result = QueryResult {
        columns: ["id", "name"]
            .iter()
            .zip(["int8", "text"])
            .map(|(name, data_type)| ColumnMeta {
                name: (*name).into(),
                data_type: data_type.into(),
                nullable: *name != "id",
            })
            .collect(),
        rows: vec![vec![CellValue::Int(7), CellValue::Text("alice".into())]],
        notices: Vec::new(),
        notice_capture: NoticeCapture::unsupported("test"),
        rows_affected: None,
        duration_ms: 0,
        truncated: false,
        total_rows: Some(1),
    };
    DataGrid::new_table(result, target, table, None, cx)
}

fn row_y() -> gpui::Pixels {
    px(HEADER_HEIGHT * theme::ui_scale() + theme::row_height() / 2.)
}

fn gutter() -> gpui::Point<gpui::Pixels> {
    point(px(18. * theme::ui_scale()), row_y())
}

/// A point inside the `name` cell: right of the fitted `id` column.
fn name_cell(grid: &Entity<DataGrid>, cx: &mut VisualTestContext) -> gpui::Point<gpui::Pixels> {
    let widths = cx.update(|_, cx| grid.read(cx).layout().portable().widths);
    point(px(36. * theme::ui_scale() + widths["id"] + 20.), row_y())
}

fn open(
    cx: &mut TestAppContext,
) -> (
    Entity<DataGrid>,
    &mut VisualTestContext,
    Rc<RefCell<Option<TableChangeRequest>>>,
) {
    cx.update(|cx| gpui_component::init(cx));
    // Cell editors are gpui-component inputs, which need a `Root` window
    // layer just like the app's real windows.
    let slot = Rc::new(RefCell::new(None));
    let inner = Rc::clone(&slot);
    let window = cx.add_window(
        move |window, cx: &mut gpui::Context<gpui_component::Root>| {
            let grid = cx.new(users_grid);
            *inner.borrow_mut() = Some(grid.clone());
            gpui_component::Root::new(grid, window, cx)
        },
    );
    let grid = slot
        .borrow_mut()
        .take()
        .expect("grid built with the window");
    let cx = VisualTestContext::from_window(*window, cx).into_mut();
    cx.run_until_parked();
    let reviewed = Rc::new(RefCell::new(None));
    let sink = Rc::clone(&reviewed);
    cx.update(|_, cx| {
        cx.subscribe(&grid, move |_, event: &DataGridEvent, _| {
            if let DataGridEvent::ReviewChanges { request, .. } = event {
                *sink.borrow_mut() = Some(request.clone());
            }
        })
        .detach();
    });
    (grid, cx, reviewed)
}

fn review(cx: &mut VisualTestContext, grid: &Entity<DataGrid>) {
    cx.update(|_, cx| grid.update(cx, |grid, cx| grid.request_review(cx)));
}

/// The single pending change, as `(kind, edited name value)`.
fn only_change(
    reviewed: &Rc<RefCell<Option<TableChangeRequest>>>,
) -> (&'static str, Option<String>) {
    let request = reviewed.borrow().clone().expect("grid emitted a review");
    assert_eq!(request.changes.len(), 1, "{:?}", request.changes);
    match &request.changes[0] {
        RowChange::Update { edits, .. } => {
            assert_eq!(edits.len(), 1);
            assert_eq!(edits[0].column, "name");
            ("update", edits[0].value.value.clone())
        }
        RowChange::Delete { .. } => ("delete", None),
        other => panic!("unexpected change {other:?}"),
    }
}

#[gpui::test]
fn backspace_in_a_double_clicked_cell_edits_its_text(cx: &mut TestAppContext) {
    let (grid, cx, reviewed) = open(cx);
    let cell = name_cell(&grid, cx);
    cx.simulate_click(cell, Modifiers::default());
    cx.simulate_event(gpui::MouseDownEvent {
        button: gpui::MouseButton::Left,
        position: cell,
        modifiers: Modifiers::default(),
        click_count: 2,
        first_mouse: false,
    });
    cx.simulate_event(gpui::MouseUpEvent {
        button: gpui::MouseButton::Left,
        position: cell,
        modifiers: Modifiers::default(),
        click_count: 2,
    });
    // The cursor starts at the end of "alice"; two backspaces leave "ali".
    cx.simulate_keystrokes("end backspace backspace enter");
    review(cx, &grid);

    assert_eq!(only_change(&reviewed), ("update", Some("ali".into())));
}

#[gpui::test]
fn backspace_on_a_selected_cell_clears_only_that_cell(cx: &mut TestAppContext) {
    let (grid, cx, reviewed) = open(cx);
    let cell = name_cell(&grid, cx);
    cx.simulate_click(cell, Modifiers::default());
    cx.simulate_keystrokes("backspace enter");
    review(cx, &grid);

    assert_eq!(only_change(&reviewed), ("update", Some(String::new())));
}

#[gpui::test]
fn backspace_on_a_gutter_selected_row_still_marks_it_for_delete(cx: &mut TestAppContext) {
    let (grid, cx, reviewed) = open(cx);
    cx.simulate_click(gutter(), Modifiers::default());
    cx.simulate_keystrokes("backspace");
    review(cx, &grid);

    assert_eq!(only_change(&reviewed).0, "delete");
}
