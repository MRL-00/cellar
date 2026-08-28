use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

use cellar_core::{
    er::{build_er_graph, ErGraph},
    schema::Database,
};
use gpui::{
    canvas, div, point, prelude::*, px, AnyElement, Context, MouseButton, MouseDownEvent,
    MouseMoveEvent, PathBuilder, ScrollWheelEvent, SharedString,
};
use gpui_component::Icon;

use super::{
    er_layout::{fit_parameters, layout_nodes, ErViewState, NodeLayout},
    CellarApp,
};
use cellar_desktop_gpui::{
    model::{ErDiagramState, ErDiagramTarget, TabKind, TableTarget, WorkspaceTab},
    theme::{
        ACCENT, BG, BORDER, FG, FG_MUTED, FG_SECONDARY, INSERT, INSET, PANEL, PANEL_RAISED, PROD,
    },
};

impl CellarApp {
    pub(super) fn open_er_diagram(
        &mut self,
        target: ErDiagramTarget,
        cx: &mut Context<Self>,
    ) -> u64 {
        let (tab_id, opened) = self.model.open_er_diagram(target.clone());
        if opened {
            self.load_er_diagram(tab_id, target, cx);
        }
        cx.notify();
        tab_id
    }

    fn load_er_diagram(&mut self, tab_id: u64, target: ErDiagramTarget, cx: &mut Context<Self>) {
        self.model.start_er_diagram(tab_id);
        let registry = Arc::clone(&self.registry);
        let runtime = Arc::clone(&self.runtime);
        cx.spawn(async move |this, cx| {
            let result = runtime
                .spawn(async move {
                    let databases = registry
                        .introspect(&target.connection_id, false)
                        .await
                        .map_err(|error| error.to_string())?;
                    build_graph(databases, &target)
                })
                .await
                .map_err(|error| format!("diagram task failed: {error}"))
                .and_then(|result| result);
            this.update(cx, |this, cx| {
                this.model.finish_er_diagram(tab_id, result);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn er_diagram_content(
        &self,
        tab: &WorkspaceTab,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let TabKind::ErDiagram { target, state } = &tab.kind else {
            unreachable!("ER renderer requires ER tab");
        };
        let tab_id = tab.id;
        let view = self.er_views.get(&tab_id).cloned().unwrap_or_default();
        let compact = view.compact;
        let refresh = target.clone();
        let fit = match state {
            ErDiagramState::Ready(graph) => fit_parameters(graph, &view),
            _ => (1., 48., 48.),
        };
        let arrange_fit = match state {
            ErDiagramState::Ready(graph) => {
                let mut arranged = view.clone();
                arranged.overrides.clear();
                fit_parameters(graph, &arranged)
            }
            _ => (1., 48., 48.),
        };
        let (nodes, edges, schemas) = match state {
            ErDiagramState::Ready(graph) => (
                graph
                    .nodes
                    .iter()
                    .filter(|node| !view.hidden_schemas.contains(&node.schema))
                    .count(),
                graph
                    .edges
                    .iter()
                    .filter(|edge| {
                        graph
                            .nodes
                            .iter()
                            .find(|node| node.id == edge.source)
                            .is_some_and(|node| !view.hidden_schemas.contains(&node.schema))
                            && graph
                                .nodes
                                .iter()
                                .find(|node| node.id == edge.target)
                                .is_some_and(|node| !view.hidden_schemas.contains(&node.schema))
                    })
                    .count(),
                graph.schemas.len(),
            ),
            _ => (0, 0, 0),
        };
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .bg(INSET)
            .child(
                div()
                    .h(px(36.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px(px(10.))
                    .border_b_1()
                    .border_color(BORDER)
                    .bg(PANEL)
                    .child(
                        Icon::empty()
                            .path("icons/diagram.svg")
                            .size(px(13.))
                            .text_color(ACCENT),
                    )
                    .child(
                        div()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("ER diagram"),
                    )
                    .child(
                        div()
                            .text_color(FG_MUTED)
                            .child(format!("{nodes} tables · {edges} relationships")),
                    )
                    .child(div().flex_1())
                    .child(toolbar_button("er-zoom-out", "−").on_click(cx.listener(
                        move |this, _, _, cx| {
                            let view = this.er_views.entry(tab_id).or_default();
                            view.zoom = (view.zoom / 1.2).max(0.1);
                            cx.notify();
                        },
                    )))
                    .child(
                        div()
                            .w(px(42.))
                            .text_center()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .text_size(px(11.))
                            .text_color(FG_MUTED)
                            .child(format!("{}%", (view.zoom * 100.).round() as u16)),
                    )
                    .child(toolbar_button("er-zoom-in", "+").on_click(cx.listener(
                        move |this, _, _, cx| {
                            let view = this.er_views.entry(tab_id).or_default();
                            view.zoom = (view.zoom * 1.2).min(2.5);
                            cx.notify();
                        },
                    )))
                    .child(toolbar_button("er-fit", "Fit").on_click(cx.listener(
                        move |this, _, _, cx| {
                            let view = this.er_views.entry(tab_id).or_default();
                            (view.zoom, view.tx, view.ty) = fit;
                            cx.notify();
                        },
                    )))
                    .child(
                        toolbar_button("er-arrange", "Auto arrange").on_click(cx.listener(
                            move |this, _, _, cx| {
                                let view = this.er_views.entry(tab_id).or_default();
                                view.overrides.clear();
                                (view.zoom, view.tx, view.ty) = arrange_fit;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        toolbar_button("er-schemas", "Schemas").on_click(cx.listener(
                            move |this, _, _, cx| {
                                let view = this.er_views.entry(tab_id).or_default();
                                view.schema_menu = !view.schema_menu;
                                cx.notify();
                            },
                        )),
                    )
                    .child(
                        div()
                            .text_size(px(10.5))
                            .text_color(FG_MUTED)
                            .child(schemas.to_string()),
                    )
                    .child(
                        toolbar_button(
                            "er-compact",
                            if compact { "Full columns" } else { "Compact" },
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            let view = this.er_views.entry(tab_id).or_default();
                            view.compact = !view.compact;
                            cx.notify();
                        })),
                    )
                    .child(
                        toolbar_button("er-refresh", "Refresh").on_click(cx.listener(
                            move |this, _, _, cx| this.load_er_diagram(tab_id, refresh.clone(), cx),
                        )),
                    ),
            )
            .child(match state {
                ErDiagramState::Loading => centered("building diagram…", false),
                ErDiagramState::Error(error) => {
                    centered_owned(format!("Could not build diagram. {error}"), true)
                }
                ErDiagramState::Ready(graph) if graph.nodes.is_empty() => {
                    centered("No tables to show.", false)
                }
                ErDiagramState::Ready(graph) => diagram_canvas(tab_id, target, graph, &view, cx),
            })
            .into_any_element()
    }
}

fn build_graph(databases: Vec<Database>, target: &ErDiagramTarget) -> Result<ErGraph, String> {
    build_er_graph(&databases, &target.database, target.schemas.as_deref()).ok_or_else(|| {
        format!(
            "database {} was not found in schema metadata",
            target.database
        )
    })
}

fn diagram_canvas(
    tab_id: u64,
    target: &ErDiagramTarget,
    graph: &ErGraph,
    view: &ErViewState,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    let nodes = graph
        .nodes
        .iter()
        .filter(|node| !view.hidden_schemas.contains(&node.schema))
        .cloned()
        .collect::<Vec<_>>();
    let node_ids = nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    let edges = graph
        .edges
        .iter()
        .filter(|edge| {
            node_ids.contains(edge.source.as_str()) && node_ids.contains(edge.target.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    for edge in &edges {
        outgoing
            .entry(edge.source.clone())
            .or_default()
            .push(format!(
                "{} → {}",
                edge.source_columns.join(", "),
                edge.target
            ));
    }
    let layouts = layout_nodes(&nodes, &outgoing, view);
    let by_id = layouts
        .iter()
        .map(|layout| (layout.node.id.as_str(), layout))
        .collect::<HashMap<_, _>>();
    let lines = edges
        .iter()
        .filter_map(|edge| {
            let source = by_id.get(edge.source.as_str())?;
            let target = by_id.get(edge.target.as_str())?;
            Some((
                source.x + source.width,
                source.y + 14.,
                target.x,
                target.y + 14.,
            ))
        })
        .collect::<Vec<_>>();
    let zoom = view.zoom;
    let tx = view.tx;
    let ty = view.ty;
    let schema_menu = view.schema_menu;
    let schemas = graph.schemas.clone();
    let hidden = view.hidden_schemas.clone();
    div()
        .id(SharedString::from(format!("er-canvas:{tab_id}")))
        .flex_1()
        .min_h_0()
        .relative()
        .overflow_hidden()
        .cursor_grab()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                let view = this.er_views.entry(tab_id).or_default();
                view.drag = Some((
                    f32::from(event.position.x),
                    f32::from(event.position.y),
                    view.tx,
                    view.ty,
                ));
                cx.notify();
            }),
        )
        .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _, cx| {
            if !event.dragging() {
                return;
            }
            let view = this.er_views.entry(tab_id).or_default();
            if let Some((id, start_x, start_y, x, y)) = view.node_drag.clone() {
                let dx = f32::from(event.position.x) - start_x;
                let dy = f32::from(event.position.y) - start_y;
                if dx.hypot(dy) > 4. {
                    view.suppress_open = Some(id.clone());
                }
                view.overrides
                    .insert(id, (x + dx / view.zoom, y + dy / view.zoom));
                cx.notify();
            } else if let Some((start_x, start_y, tx, ty)) = view.drag {
                view.tx = tx + f32::from(event.position.x) - start_x;
                view.ty = ty + f32::from(event.position.y) - start_y;
                cx.notify();
            }
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(move |this, _, _, _| {
                this.er_views.entry(tab_id).or_default().drag = None;
                this.er_views.entry(tab_id).or_default().node_drag = None;
            }),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(move |this, _, _, _| {
                this.er_views.entry(tab_id).or_default().drag = None;
                this.er_views.entry(tab_id).or_default().node_drag = None;
            }),
        )
        .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
            let dy = f32::from(event.delta.pixel_delta(px(16.)).y);
            let view = this.er_views.entry(tab_id).or_default();
            view.zoom = (view.zoom * (-dy * 0.0015).exp()).clamp(0.1, 2.5);
            cx.notify();
        }))
        .child(
            canvas(
                |_, _, _| (),
                move |bounds, _, window, _| {
                    for (sx, sy, ex, ey) in lines {
                        let sx = sx * zoom + tx;
                        let sy = sy * zoom + ty;
                        let ex = ex * zoom + tx;
                        let ey = ey * zoom + ty;
                        let mid = (sx + ex) / 2.;
                        let mut path = PathBuilder::stroke(px(1.));
                        path.move_to(point(bounds.origin.x + px(sx), bounds.origin.y + px(sy)));
                        path.cubic_bezier_to(
                            point(bounds.origin.x + px(ex), bounds.origin.y + px(ey)),
                            point(bounds.origin.x + px(mid), bounds.origin.y + px(sy)),
                            point(bounds.origin.x + px(mid), bounds.origin.y + px(ey)),
                        );
                        if let Ok(path) = path.build() {
                            window.paint_path(path, FG_MUTED);
                        }
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .children(layouts.into_iter().map(|layout| {
            node_card(
                tab_id,
                target,
                &layout,
                outgoing.get(&layout.node.id).cloned().unwrap_or_default(),
                view,
                cx,
            )
        }))
        .when(schema_menu, |element| {
            element.child(
                div()
                    .absolute()
                    .right(px(10.))
                    .top(px(10.))
                    .w(px(220.))
                    .rounded(px(5.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(PANEL)
                    .shadow_lg()
                    .p_1()
                    .children(schemas.into_iter().map(|schema| {
                        let checked = !hidden.contains(&schema);
                        let toggle = schema.clone();
                        div()
                            .id(SharedString::from(format!("er-schema:{schema}")))
                            .tab_index(0)
                            .cursor_pointer()
                            .h(px(26.))
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_2()
                            .rounded(px(3.))
                            .hover(|style| style.bg(PANEL_RAISED))
                            .child(
                                div()
                                    .w(px(12.))
                                    .text_color(if checked { ACCENT } else { FG_MUTED })
                                    .child(if checked { "✓" } else { "" }),
                            )
                            .child(schema)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                let view = this.er_views.entry(tab_id).or_default();
                                if !view.hidden_schemas.remove(&toggle) {
                                    view.hidden_schemas.insert(toggle.clone());
                                }
                                cx.notify();
                            }))
                    })),
            )
        })
        .into_any_element()
}

fn node_card(
    tab_id: u64,
    target: &ErDiagramTarget,
    layout: &NodeLayout,
    relationships: Vec<String>,
    view: &ErViewState,
    cx: &mut Context<CellarApp>,
) -> AnyElement {
    let node = &layout.node;
    let open = TableTarget {
        connection_id: target.connection_id.clone(),
        database: target.database.clone(),
        schema: node.schema.clone(),
        table: node.name.clone(),
    };
    let columns = node
        .columns
        .iter()
        .filter(|column| !view.compact || column.is_primary_key || column.is_foreign_key)
        .take(layout.visible_columns)
        .collect::<Vec<_>>();
    let candidates = node
        .columns
        .iter()
        .filter(|column| !view.compact || column.is_primary_key || column.is_foreign_key)
        .count();
    let hidden = candidates.saturating_sub(columns.len());
    let zoom = view.zoom;
    let drag_id = node.id.clone();
    let open_id = node.id.clone();
    let drag_x = layout.x;
    let drag_y = layout.y;
    div()
        .id(SharedString::from(format!("er-node:{}", node.id)))
        .tab_index(0)
        .absolute()
        .left(px(layout.x * zoom + view.tx))
        .top(px(layout.y * zoom + view.ty))
        .w(px(layout.width * zoom))
        .h(px(layout.height * zoom))
        .cursor_pointer()
        .rounded(px(5.))
        .border_1()
        .border_color(BORDER)
        .bg(BG)
        .shadow_sm()
        .hover(|style| style.border_color(ACCENT))
        .child(
            div()
                .h(px(28. * zoom))
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .border_b_1()
                .border_color(BORDER)
                .child(
                    Icon::empty()
                        .path("icons/table.svg")
                        .size(px(11. * zoom.clamp(0.65, 1.5)))
                        .text_color(ACCENT),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .font_family(cellar_desktop_gpui::theme::mono_font())
                        .text_size(px(12. * zoom.clamp(0.65, 1.5)))
                        .text_color(FG)
                        .child(format!("{}.{}", node.schema, node.name)),
                )
                .when_some(node.row_count, |element, rows| {
                    element.child(
                        div()
                            .text_size(px(10.))
                            .text_color(FG_MUTED)
                            .child(rows.to_string()),
                    )
                }),
        )
        .children(columns.into_iter().map(|column| {
            div()
                .h(px(20. * zoom))
                .flex()
                .items_center()
                .gap_2()
                .px_2()
                .font_family(cellar_desktop_gpui::theme::mono_font())
                .text_size(px(11. * zoom.clamp(0.65, 1.5)))
                .child(
                    div()
                        .w(px(18.))
                        .text_color(if column.is_primary_key {
                            INSERT
                        } else if column.is_foreign_key {
                            ACCENT
                        } else {
                            FG_MUTED
                        })
                        .child(if column.is_primary_key {
                            "PK"
                        } else if column.is_foreign_key {
                            "FK"
                        } else {
                            ""
                        }),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .text_color(FG_SECONDARY)
                        .child(column.name.clone()),
                )
                .child(
                    div()
                        .max_w(px(90.))
                        .truncate()
                        .text_color(FG_MUTED)
                        .child(column.data_type.clone()),
                )
        }))
        .when(hidden > 0, |element| {
            let node_id = node.id.clone();
            element.child(
                div()
                    .id(SharedString::from(format!("er-expand:{node_id}")))
                    .tab_index(0)
                    .cursor_pointer()
                    .px_2()
                    .py_1()
                    .text_size(px(10.5))
                    .text_color(FG_MUTED)
                    .child(format!("+{hidden} more columns"))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.er_views
                            .entry(tab_id)
                            .or_default()
                            .expanded
                            .insert(node_id.clone());
                        cx.notify();
                    })),
            )
        })
        .when(!relationships.is_empty(), |element| {
            element.child(
                div()
                    .border_t_1()
                    .border_color(BORDER)
                    .px_2()
                    .py_1()
                    .children(relationships.into_iter().map(|relationship| {
                        div()
                            .truncate()
                            .text_size(px(10.5))
                            .text_color(FG_MUTED)
                            .child(relationship)
                    })),
            )
        })
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                cx.stop_propagation();
                this.er_views.entry(tab_id).or_default().node_drag = Some((
                    drag_id.clone(),
                    f32::from(event.position.x),
                    f32::from(event.position.y),
                    drag_x,
                    drag_y,
                ));
            }),
        )
        .on_click(cx.listener(move |this, _, window, cx| {
            let view = this.er_views.entry(tab_id).or_default();
            if view.suppress_open.as_deref() == Some(open_id.as_str()) {
                view.suppress_open = None;
                return;
            }
            this.open_table(open.clone(), window, cx)
        }))
        .into_any_element()
}

fn toolbar_button(id: &'static str, label: &'static str) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .tab_index(0)
        .cursor_pointer()
        .h(px(23.))
        .flex()
        .items_center()
        .rounded(px(4.))
        .border_1()
        .border_color(BORDER)
        .bg(PANEL_RAISED)
        .px_2()
        .text_size(px(12.))
        .text_color(FG_SECONDARY)
        .hover(|style| style.text_color(FG))
        .child(label)
}

fn centered(message: &'static str, warn: bool) -> AnyElement {
    centered_owned(message.into(), warn)
}

fn centered_owned(message: String, warn: bool) -> AnyElement {
    div()
        .flex_1()
        .flex()
        .items_center()
        .justify_center()
        .text_color(if warn { PROD } else { FG_MUTED })
        .child(message)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use cellar_core::schema::Database;

    use super::{build_graph, ErDiagramTarget};

    #[test]
    fn missing_database_is_a_visible_diagram_error() {
        let error = build_graph(
            Vec::<Database>::new(),
            &ErDiagramTarget {
                connection_id: "one".into(),
                database: "missing".into(),
                schemas: None,
            },
        )
        .unwrap_err();
        assert!(error.contains("missing"));
    }
}
