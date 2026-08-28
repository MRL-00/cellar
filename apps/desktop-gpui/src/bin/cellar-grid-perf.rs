use std::time::Instant;

use cellar_core::{
    query::{NoticeCapture, QueryResult},
    value::{CellValue, ColumnMeta},
};
use gpui::{
    px, size, App, AppContext, Application, Bounds, Context, Entity, IntoElement, Render, Window,
    WindowBounds, WindowOptions,
};

use cellar_desktop_gpui::grid::DataGrid;

const ROWS: usize = 10_000;
const COLUMNS: usize = 500;
const WARMUP_FRAMES: usize = 30;
const MEASURED_FRAMES: usize = 240;

struct GridPerf {
    grid: Entity<DataGrid>,
    last_frame: Instant,
    frame: usize,
    intervals_ms: Vec<f64>,
}

impl GridPerf {
    fn new(grid: Entity<DataGrid>) -> Self {
        Self {
            grid,
            last_frame: Instant::now(),
            frame: 0,
            intervals_ms: Vec::with_capacity(MEASURED_FRAMES),
        }
    }

    fn tick(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let now = Instant::now();
        if self.frame >= WARMUP_FRAMES {
            self.intervals_ms
                .push((now - self.last_frame).as_secs_f64() * 1_000.);
        }
        self.last_frame = now;

        if self.intervals_ms.len() == MEASURED_FRAMES {
            let mean_ms = self.intervals_ms.iter().sum::<f64>() / MEASURED_FRAMES as f64;
            let mut sorted = self.intervals_ms.clone();
            sorted.sort_by(f64::total_cmp);
            let p95_ms = sorted[MEASURED_FRAMES * 95 / 100];
            let fps = 1_000. / mean_ms;
            let passed = fps >= 60. && p95_ms <= 20.;
            println!(
                "grid_rows={ROWS} grid_columns={COLUMNS} mean_fps={fps:.1} p95_frame_ms={p95_ms:.2} PERF_RESULT={}",
                if passed { "PASS" } else { "FAIL" }
            );
            cx.quit();
            return;
        }

        let row = self.frame.wrapping_mul(37) % ROWS;
        let column = self.frame.wrapping_mul(13) % COLUMNS;
        self.grid
            .update(cx, |grid, cx| grid.scroll_to_cell(row, column, cx));
        self.frame += 1;
        schedule_next(cx.weak_entity(), window, cx);
    }
}

impl Render for GridPerf {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.grid.clone()
    }
}

fn schedule_next(perf: gpui::WeakEntity<GridPerf>, window: &mut Window, cx: &mut App) {
    window.on_next_frame(move |window, cx| {
        perf.update(cx, |perf, cx| perf.tick(window, cx)).ok();
    });
    cx.refresh_windows();
}

fn result() -> QueryResult {
    let columns = (0..COLUMNS)
        .map(|column| ColumnMeta {
            name: format!("column_{}", column + 1),
            data_type: "text".into(),
            nullable: true,
        })
        .collect();
    let row = vec![CellValue::Null; COLUMNS];
    QueryResult {
        columns,
        rows: vec![row; ROWS],
        notices: Vec::new(),
        notice_capture: NoticeCapture::unsupported("performance workload"),
        rows_affected: None,
        duration_ms: 0,
        truncated: false,
        total_rows: Some(ROWS as u64),
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(
            WindowOptions {
                focus: true,
                window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                    None,
                    size(px(1440.), px(900.)),
                    cx,
                ))),
                ..Default::default()
            },
            |window, cx| {
                let grid = cx.new(|cx| DataGrid::new(result(), cx));
                let perf = cx.new(|_| GridPerf::new(grid));
                schedule_next(perf.downgrade(), window, cx);
                perf
            },
        )
        .expect("open grid performance window");
        cx.activate(true);
    });
}
