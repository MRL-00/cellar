use cellar_runtime::export::ExportFormat;
use gpui::{div, prelude::*, px, AnyElement, Context, MouseButton, SharedString};

use super::CellarApp;
use cellar_desktop_gpui::theme::{
    accent_soft, ui_px, ACCENT, BORDER_STRONG, FG_SECONDARY, PANEL_MUTED,
};

impl CellarApp {
    pub(super) fn bottom_export_menu(
        &self,
        grid: gpui::Entity<cellar_desktop_gpui::grid::DataGrid>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id("bottom-export-backdrop")
            .absolute()
            .inset_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.bottom_export_menu = false;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .id("bottom-export-menu")
                    .tab_group()
                    .absolute()
                    .right(px(34.))
                    .top(px(27.))
                    .min_w(ui_px(176.))
                    .py(ui_px(4.))
                    .rounded(ui_px(6.))
                    .border_1()
                    .border_color(BORDER_STRONG)
                    .bg(PANEL_MUTED)
                    .shadow_lg()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .children(
                        [
                            ("CSV", ExportFormat::Csv),
                            ("TSV", ExportFormat::Tsv),
                            ("JSON", ExportFormat::Json),
                            ("SQL INSERT", ExportFormat::Sql),
                        ]
                        .map(|(label, format)| {
                            let export_grid = grid.clone();
                            div()
                                .id(SharedString::from(format!("bottom-export-{label}")))
                                .tab_index(0)
                                .cursor_pointer()
                                .h(ui_px(28.))
                                .flex()
                                .items_center()
                                .px(ui_px(10.))
                                .text_color(FG_SECONDARY)
                                .hover(|style| style.bg(accent_soft()).text_color(ACCENT))
                                .child(format!("Export as {label}"))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.bottom_export_menu = false;
                                    export_grid
                                        .update(cx, |grid, cx| grid.begin_export(format, cx));
                                    cx.notify();
                                }))
                        }),
                    ),
            )
            .into_any_element()
    }
}
