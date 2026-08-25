//! PROTOTYPE — throw away after deciding whether Cellar should migrate to GPUI.
//! Three workloads are switchable from the bottom bar: 50×500, 500×500, 500×10,000.

use std::ops::Range;

use gpui::{
    div, prelude::*, px, rgb, size, uniform_list, App, Application, Bounds, Context, IntoElement,
    Render, RenderOnce, ScrollHandle, ScrollWheelEvent, UniformListScrollHandle, Window,
    WindowBounds, WindowOptions,
};

const CELL_WIDTH: f32 = 120.;

#[derive(Clone, Copy)]
struct Workload {
    name: &'static str,
    rows: usize,
    columns: usize,
}

const WORKLOADS: [Workload; 3] = [
    Workload {
        name: "A — ordinary",
        rows: 500,
        columns: 50,
    },
    Workload {
        name: "B — wide",
        rows: 500,
        columns: 500,
    },
    Workload {
        name: "C — stress",
        rows: 10_000,
        columns: 500,
    },
];

#[derive(IntoElement)]
struct GridRow {
    row: usize,
    columns: Range<usize>,
    total_columns: usize,
}

impl RenderOnce for GridRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .h(px(23.))
            .w(px(self.total_columns as f32 * CELL_WIDTH + 44.))
            .bg(if self.row.is_multiple_of(2) {
                rgb(0x111318)
            } else {
                rgb(0x15181e)
            })
            .border_b_1()
            .border_color(rgb(0x242832))
            .child(
                div()
                    .w(px(44. + self.columns.start as f32 * CELL_WIDTH))
                    .flex_shrink_0()
                    .px_2()
                    .text_color(rgb(0x6f7787))
                    .when(self.columns.start == 0, |element| {
                        element.child((self.row + 1).to_string())
                    }),
            )
            .children(self.columns.clone().map(|column| {
                div()
                    .w(px(CELL_WIDTH))
                    .flex_shrink_0()
                    .px_2()
                    .border_l_1()
                    .border_color(rgb(0x242832))
                    .text_color(if column.is_multiple_of(5) {
                        rgb(0x8fb8ff)
                    } else {
                        rgb(0xc7ccd6)
                    })
                    .whitespace_nowrap()
                    .truncate()
                    .child(format!("r{}_c{}", self.row + 1, column + 1))
            }))
            .child(
                div()
                    .w(px(
                        (self.total_columns - self.columns.end) as f32 * CELL_WIDTH
                    ))
                    .flex_shrink_0(),
            )
    }
}

struct GridPrototype {
    workload: usize,
    visible_rows: Range<usize>,
    vertical_scroll: UniformListScrollHandle,
    horizontal_scroll: ScrollHandle,
}

impl GridPrototype {
    fn select(&mut self, workload: usize, cx: &mut Context<Self>) {
        self.workload = workload;
        self.vertical_scroll = UniformListScrollHandle::new();
        self.horizontal_scroll = ScrollHandle::new();
        cx.notify();
    }

    fn visible_columns(&self, total: usize) -> Range<usize> {
        let offset = (-f32::from(self.horizontal_scroll.offset().x)).max(0.);
        let viewport = f32::from(self.horizontal_scroll.bounds().size.width).max(1440.);
        let first = ((offset / CELL_WIDTH) as usize)
            .saturating_sub(2)
            .min(total);
        let last = (((offset + viewport) / CELL_WIDTH).ceil() as usize + 2).min(total);
        first..last
    }

    fn header(total_columns: usize, columns: Range<usize>) -> impl IntoElement {
        div()
            .flex()
            .h(px(28.))
            .w(px(total_columns as f32 * CELL_WIDTH + 44.))
            .bg(rgb(0x1d2129))
            .border_b_1()
            .border_color(rgb(0x343a46))
            .child(
                div()
                    .w(px(44. + columns.start as f32 * CELL_WIDTH))
                    .flex_shrink_0()
                    .px_2()
                    .when(columns.start == 0, |element| element.child("#")),
            )
            .children(columns.clone().map(|column| {
                div()
                    .w(px(CELL_WIDTH))
                    .flex_shrink_0()
                    .px_2()
                    .border_l_1()
                    .border_color(rgb(0x343a46))
                    .text_color(rgb(0x9aa3b3))
                    .child(format!("column_{}", column + 1))
            }))
            .child(
                div()
                    .w(px((total_columns - columns.end) as f32 * CELL_WIDTH))
                    .flex_shrink_0(),
            )
    }
}

impl Render for GridPrototype {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let workload = WORKLOADS[self.workload];
        let visible_columns = self.visible_columns(workload.columns);
        let rendered_rows = self
            .visible_rows
            .end
            .saturating_sub(self.visible_rows.start);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x0c0e12))
            .text_color(rgb(0xd9dde5))
            .font_family("JetBrains Mono")
            .text_size(px(12.))
            .child(
                div()
                    .h(px(42.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .border_b_1()
                    .border_color(rgb(0x2b303a))
                    .child("Cellar GPUI grid prototype")
                    .child(format!(
                        "{} rows × {} columns · rendering {} visible rows",
                        workload.rows, workload.columns, rendered_rows
                    )),
            )
            .child(
                div()
                    .id("grid-scroller")
                    .flex_1()
                    .overflow_x_scroll()
                    .track_scroll(&self.horizontal_scroll)
                    .on_scroll_wheel(cx.listener(|_, _: &ScrollWheelEvent, _, cx| cx.notify()))
                    .child(Self::header(workload.columns, visible_columns.clone()))
                    .child(
                        uniform_list(
                            "prototype-grid",
                            workload.rows,
                            cx.processor(move |this, range: Range<usize>, _, _| {
                                this.visible_rows = range.clone();
                                range
                                    .map(|row| GridRow {
                                        row,
                                        columns: visible_columns.clone(),
                                        total_columns: workload.columns,
                                    })
                                    .collect::<Vec<_>>()
                            }),
                        )
                        .h_full()
                        .w(px(workload.columns as f32 * CELL_WIDTH + 44.))
                        .track_scroll(self.vertical_scroll.clone()),
                    ),
            )
            .child(
                div()
                    .h(px(48.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .border_t_1()
                    .border_color(rgb(0x2b303a))
                    .children(WORKLOADS.into_iter().enumerate().map(|(index, option)| {
                        let active = index == self.workload;
                        div()
                            .id(("workload", index))
                            .cursor_pointer()
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .border_1()
                            .border_color(if active { rgb(0x73a8ff) } else { rgb(0x39404d) })
                            .bg(if active { rgb(0x21375b) } else { rgb(0x171a20) })
                            .child(option.name)
                            .on_click(cx.listener(move |this, _, _, cx| this.select(index, cx)))
                    })),
            )
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
            |_, cx| {
                cx.new(|_| GridPrototype {
                    workload: 1,
                    visible_rows: 0..0,
                    vertical_scroll: UniformListScrollHandle::new(),
                    horizontal_scroll: ScrollHandle::new(),
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
