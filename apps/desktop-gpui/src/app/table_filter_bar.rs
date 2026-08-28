use cellar_core::query::{SortDirection, TableFilterOperator, TableSortClause};
use gpui::{
    canvas, div, prelude::*, px, AnyElement, Bounds, ClickEvent, Context, Div, Entity, Pixels,
    SharedString,
};
use gpui_component::{input::InputState, Icon};

use super::CellarApp;
use cellar_desktop_gpui::{
    model::{TablePage, TableTarget},
    theme::{ACCENT, ACCENT_FG, BORDER, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL, PANEL_RAISED},
    widgets::compact_input,
};

const BAR_H: f32 = 32.;
const FIELD_H: f32 = 22.;
const TYPE: f32 = 12.;
const LEAD: f32 = 16.;
const ICON: f32 = 11.;
const RADIUS: f32 = 3.;

fn field() -> Div {
    div()
        .h(px(FIELD_H))
        .min_h(px(FIELD_H))
        .max_h(px(FIELD_H))
        .flex_shrink_0()
        .flex()
        .items_center()
        .overflow_hidden()
        .rounded(px(RADIUS))
        .border_1()
        .border_color(BORDER)
        .bg(INSET)
        .px(px(7.))
        .text_size(px(TYPE))
        .line_height(px(LEAD))
}

fn bar_input(state: &Entity<InputState>) -> gpui_component::input::Input {
    compact_input(state).h(px(LEAD)).max_h(px(LEAD))
}

fn bar_icon(path: &'static str) -> Icon {
    Icon::empty().path(path).size(px(ICON))
}

fn remember_bounds(
    app: gpui::WeakEntity<CellarApp>,
    write: fn(&mut CellarApp, Bounds<Pixels>),
) -> impl IntoElement {
    canvas(
        move |bounds, _, cx| {
            app.update(cx, |this, _| write(this, bounds)).ok();
        },
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

impl CellarApp {
    pub(super) fn table_filter_bar(
        &self,
        tab_id: u64,
        target: &TableTarget,
        page: TablePage,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let input = self.table_filter_inputs.get(&tab_id)?.clone();
        let quick_input = self.table_quick_filter_inputs.get(&tab_id)?.clone();
        let quick_column = self
            .model
            .table(target)?
            .columns
            .get(*self.table_quick_filter_columns.get(&tab_id).unwrap_or(&0))
            .filter(|column| super::table_quick_filter::is_text_type(&column.data_type))
            .map(|column| column.name.clone());
        let column = self
            .model
            .table(target)?
            .columns
            .get(*self.table_filter_columns.get(&tab_id).unwrap_or(&0))?
            .name
            .clone();
        let sort = self.table_sorts.get(&tab_id).cloned();
        Some(
            self.filter_bar(
                tab_id,
                input,
                quick_input,
                quick_column,
                column,
                sort,
                page.rows,
                self.table_filters.get(&tab_id).map(Vec::len).unwrap_or(0),
                cx,
            )
            .into_any_element(),
        )
    }

    fn filter_bar(
        &self,
        tab_id: u64,
        input: Entity<InputState>,
        quick_input: Entity<InputState>,
        quick_column: Option<String>,
        column: String,
        sort: Option<TableSortClause>,
        row_count: u32,
        active_count: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let filters = self.table_filters.get(&tab_id).cloned().unwrap_or_default();
        let composing = self.table_filter_composers.contains(&tab_id);
        let operator = self
            .table_filter_operators
            .get(&tab_id)
            .copied()
            .unwrap_or(TableFilterOperator::Equals);
        let mut chips = div()
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .gap(px(6.));
        for (index, filter) in filters.iter().enumerate() {
            let value = filter.value.as_deref().unwrap_or("");
            let text = if value.is_empty() {
                format!("{} {}", filter.column, operator_label(filter.operator))
            } else {
                format!(
                    "{} {} {}",
                    filter.column,
                    operator_label(filter.operator),
                    value
                )
            };
            chips = chips.child(
                field()
                    .flex_shrink_0()
                    .max_w(px(360.))
                    .border_color(cellar_desktop_gpui::theme::accent(0.32))
                    .bg(cellar_desktop_gpui::theme::accent(0.14))
                    .font_family(cellar_desktop_gpui::theme::mono_font())
                    .text_color(FG)
                    .gap(px(4.))
                    .child(
                        div()
                            .id(SharedString::from(format!("filter-edit:{tab_id}:{index}")))
                            .tab_index(0)
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .child(text)
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.edit_table_filter(tab_id, index, window, cx);
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "filter-remove:{tab_id}:{index}"
                            )))
                            .tab_index(0)
                            .size(px(14.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(2.))
                            .hover(|style| style.bg(gpui::rgba(0x00000033)))
                            .child(bar_icon("icons/close.svg").text_color(FG_MUTED))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.remove_table_filter(tab_id, index, cx);
                            })),
                    ),
            );
        }
        chips = if composing {
            let apply = tab_id;
            chips.child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .rounded(px(RADIUS))
                    .border_1()
                    .border_color(FG_MUTED)
                    .bg(PANEL_RAISED)
                    .px(px(4.))
                    .child(
                        field()
                            .id(SharedString::from(format!("filter-column:{tab_id}")))
                            .tab_index(0)
                            .cursor_pointer()
                            .max_w(px(150.))
                            .text_color(FG)
                            .child(div().truncate().child(column))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.cycle_filter_column(tab_id, cx);
                            })),
                    )
                    .child(
                        field()
                            .id(SharedString::from(format!("filter-operator:{tab_id}")))
                            .tab_index(0)
                            .cursor_pointer()
                            .max_w(px(104.))
                            .text_color(FG)
                            .child(operator_label(operator))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.cycle_filter_operator(tab_id, cx);
                            })),
                    )
                    .child(
                        field()
                            .w(px(132.))
                            .px(px(6.))
                            .child(bar_input(&input).flex_1()),
                    )
                    .child(
                        field()
                            .id(SharedString::from(format!("filter-apply:{tab_id}")))
                            .tab_index(0)
                            .cursor_pointer()
                            .border_color(ACCENT)
                            .bg(ACCENT)
                            .text_color(ACCENT_FG)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("apply")
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.apply_table_filter(apply, window, cx);
                            })),
                    ),
            )
        } else {
            chips.child(
                field()
                    .id(SharedString::from(format!("filter-add:{tab_id}")))
                    .tab_index(0)
                    .cursor_pointer()
                    .gap(px(4.))
                    .text_color(FG_SECONDARY)
                    .child(bar_icon("icons/plus.svg").text_color(FG_MUTED))
                    .child("add")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_table_filter(tab_id, window, cx);
                    })),
            )
        };
        let clear = tab_id;
        let quick_active = self.table_quick_filters.contains_key(&tab_id)
            || !quick_input.read(cx).value().trim().is_empty();
        let total_active = active_count + usize::from(quick_active);
        let sort_column = sort.as_ref().map_or("—", |sort| sort.column.as_str());
        let sort_icon = if matches!(
            sort.as_ref().map(|sort| sort.direction),
            Some(SortDirection::Desc)
        ) {
            "icons/sort-desc.svg"
        } else {
            "icons/sort-asc.svg"
        };
        div()
            .h(px(BAR_H))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap(px(8.))
            .px(px(10.))
            .bg(PANEL)
            .border_b_1()
            .border_color(BORDER)
            .text_size(px(TYPE))
            .line_height(px(LEAD))
            .child(
                div()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .pr(px(8.))
                    .border_r_1()
                    .border_color(BORDER)
                    .child(bar_icon("icons/grid-search.svg").text_color(FG_MUTED))
                    .child(
                        field()
                            .w(px(180.))
                            .px(px(6.))
                            .child(bar_input(&quick_input).flex_1()),
                    )
                    .when_some(quick_column, |element, quick_column| {
                        let app = cx.weak_entity();
                        element.child(
                            div()
                                .id(SharedString::from(format!("quick-column:{tab_id}")))
                                .tab_index(0)
                                .relative()
                                .h(px(FIELD_H))
                                .flex_shrink_0()
                                .cursor_pointer()
                                .child(
                                    field()
                                        .max_w(px(140.))
                                        .gap(px(4.))
                                        .text_color(FG)
                                        .child(div().min_w_0().truncate().child(quick_column))
                                        .child(
                                            bar_icon("icons/chevron-down.svg").text_color(FG_MUTED),
                                        ),
                                )
                                .child(remember_bounds(app, |this, bounds| {
                                    this.quick_column_trigger_bounds = Some(bounds);
                                }))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.open_quick_column_menu(tab_id, cx);
                                })),
                        )
                    })
                    .when(quick_active, |element| {
                        element.child(
                            div()
                                .id(SharedString::from(format!("quick-clear:{tab_id}")))
                                .tab_index(0)
                                .cursor_pointer()
                                .size(px(FIELD_H))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(RADIUS))
                                .child(bar_icon("icons/close.svg").text_color(FG_MUTED))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.clear_quick_filter(tab_id, window, cx);
                                })),
                        )
                    }),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .h(px(FIELD_H))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(FG_SECONDARY)
                    .child(bar_icon("icons/filter.svg").text_color(ACCENT))
                    .child("where")
                    .when(total_active > 0, |element| {
                        element.child(
                            field()
                                .px(px(5.))
                                .text_color(FG_MUTED)
                                .child(format!("{total_active} active")),
                        )
                    }),
            )
            .child(chips)
            .when(active_count > 1, |element| {
                element.child(
                    field()
                        .id(SharedString::from(format!("filter-clear:{tab_id}")))
                        .tab_index(0)
                        .cursor_pointer()
                        .text_color(FG_MUTED)
                        .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                        .child("clear")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.clear_table_filter(clear, window, cx);
                        })),
                )
            })
            .child(
                div()
                    .flex_shrink_0()
                    .h(px(FIELD_H))
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .pl(px(8.))
                    .border_l_1()
                    .border_color(BORDER)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(FG_SECONDARY)
                            .child(
                                bar_icon(sort_icon).text_color(if sort.is_some() {
                                    ACCENT
                                } else {
                                    FG_MUTED
                                }),
                            )
                            .child("order by"),
                    )
                    .child(
                        field()
                            .id(SharedString::from(format!("sort-column:{tab_id}")))
                            .tab_index(0)
                            .cursor_pointer()
                            .max_w(px(150.))
                            .min_w(px(42.))
                            .child(div().truncate().child(sort_column.to_owned()))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.cycle_toolbar_sort_column(tab_id, cx);
                            })),
                    )
                    .when_some(sort, |element, sort| {
                        element.child(
                            field()
                                .id(SharedString::from(format!("sort-direction:{tab_id}")))
                                .tab_index(0)
                                .cursor_pointer()
                                .font_family(cellar_desktop_gpui::theme::mono_font())
                                .text_color(FG_MUTED)
                                .child(match sort.direction {
                                    SortDirection::Asc => "ASC",
                                    SortDirection::Desc => "DESC",
                                })
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.toggle_toolbar_sort_direction(tab_id, cx);
                                })),
                        )
                    }),
            )
            .child(self.filter_preset_control(tab_id, cx))
            .child(
                div()
                    .ml_auto()
                    .flex_shrink_0()
                    .h(px(FIELD_H))
                    .flex()
                    .items_center()
                    .gap(px(4.))
                    .font_family(cellar_desktop_gpui::theme::mono_font())
                    .child(div().text_color(FG).child(row_count.to_string()))
                    .child(div().text_color(FG_MUTED).child("/"))
                    .child(div().text_color(FG_SECONDARY).child(row_count.to_string())),
            )
    }

    fn filter_preset_control(&self, tab_id: u64, cx: &mut Context<Self>) -> AnyElement {
        let active = self.active_filter_preset(tab_id);
        let draft = self
            .table_preset_draft
            .as_ref()
            .filter(|draft| draft.tab_id == tab_id)
            .map(|draft| draft.input.clone());
        div()
            .flex_shrink_0()
            .h(px(FIELD_H))
            .flex()
            .items_center()
            .gap(px(6.))
            .pl(px(8.))
            .border_l_1()
            .border_color(BORDER)
            .when_some(draft, |element, input| {
                element
                    .child(
                        field()
                            .w(px(132.))
                            .px(px(6.))
                            .child(bar_input(&input).flex_1()),
                    )
                    .child(
                        field()
                            .id(SharedString::from(format!("preset-commit:{tab_id}")))
                            .tab_index(0)
                            .cursor_pointer()
                            .border_color(ACCENT)
                            .bg(ACCENT)
                            .text_color(ACCENT_FG)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("save")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.commit_filter_preset(cx);
                            })),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("preset-cancel:{tab_id}")))
                            .tab_index(0)
                            .cursor_pointer()
                            .size(px(FIELD_H))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(bar_icon("icons/close.svg").text_color(FG_MUTED))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel_filter_preset_draft(cx);
                            })),
                    )
            })
            .when(
                self.table_preset_draft
                    .as_ref()
                    .is_none_or(|draft| draft.tab_id != tab_id),
                |element| {
                    let label = active.clone().unwrap_or_else(|| "Presets".into());
                    element.child({
                        let app = cx.weak_entity();
                        div()
                            .id(SharedString::from(format!("preset-trigger:{tab_id}")))
                            .tab_index(0)
                            .relative()
                            .h(px(FIELD_H))
                            .flex_shrink_0()
                            .cursor_pointer()
                            .child(
                                field()
                                    .max_w(px(190.))
                                    .gap(px(5.))
                                    .border_color(if active.is_some() {
                                        cellar_desktop_gpui::theme::accent(0.32)
                                    } else {
                                        BORDER.rgba()
                                    })
                                    .text_color(if active.is_some() { ACCENT } else { FG_SECONDARY })
                                    .child(bar_icon("icons/bookmark.svg").text_color(
                                        if active.is_some() {
                                            ACCENT
                                        } else {
                                            FG_MUTED
                                        },
                                    ))
                                    .child(div().min_w_0().truncate().child(label))
                                    .child(
                                        bar_icon("icons/chevron-down.svg").text_color(FG_MUTED),
                                    ),
                            )
                            .child(remember_bounds(app, |this, bounds| {
                                this.preset_trigger_bounds = Some(bounds);
                            }))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                this.open_filter_preset_menu(tab_id, cx);
                            }))
                    })
                },
            )
            .into_any_element()
    }
}

fn operator_label(operator: TableFilterOperator) -> &'static str {
    match operator {
        TableFilterOperator::Equals => "=",
        TableFilterOperator::NotEquals => "≠",
        TableFilterOperator::Contains => "contains",
        TableFilterOperator::NotContains => "not contains",
        TableFilterOperator::StartsWith => "starts with",
        TableFilterOperator::EndsWith => "ends with",
        TableFilterOperator::Like => "like",
        TableFilterOperator::IsNull => "is null",
        TableFilterOperator::IsNotNull => "is not null",
        TableFilterOperator::GreaterThan => ">",
        TableFilterOperator::GreaterThanOrEqual => "≥",
        TableFilterOperator::LessThan => "<",
        TableFilterOperator::LessThanOrEqual => "≤",
    }
}
