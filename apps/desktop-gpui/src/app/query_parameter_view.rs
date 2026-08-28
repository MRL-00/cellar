use gpui::{div, prelude::*, px, AnyElement, Context, SharedString};
use gpui_component::{input::Input, scroll::ScrollableElement, Icon};

use super::{
    query_params::{parameter_value, ParamKind, QueryParameterInput},
    query_widgets::query_keycap,
    CellarApp,
};
use cellar_desktop_gpui::theme::{
    ACCENT, ACCENT_FG, BORDER, FG, FG_MUTED, FG_SECONDARY, INSET, PANEL, PANEL_RAISED, PROD,
};

impl CellarApp {
    pub(super) fn query_parameter_panel(
        &self,
        tab_id: u64,
        inputs: &[QueryParameterInput],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .flex_shrink_0()
            .max_h(px(320.))
            .flex()
            .flex_col()
            .bg(PANEL)
            .border_b_1()
            .border_color(BORDER)
            .font_family(cellar_desktop_gpui::theme::mono_font())
            .child(self.parameter_header(tab_id, cx))
            .child(
                div()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .px(px(10.))
                    .pb_2()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .children(
                        inputs
                            .iter()
                            .enumerate()
                            .map(|(index, input)| self.parameter_row(tab_id, index, input, cx)),
                    ),
            )
            .child(
                div()
                    .h(px(38.))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px(px(10.))
                    .child(
                        div()
                            .id(SharedString::from(format!("run-query-params:{tab_id}")))
                            .tab_index(0)
                            .cursor_pointer()
                            .h(px(22.))
                            .flex()
                            .items_center()
                            .gap(px(5.))
                            .rounded(px(4.))
                            .border_1()
                            .border_color(ACCENT)
                            .bg(ACCENT)
                            .px_2()
                            .text_color(ACCENT_FG)
                            .child(Icon::empty().path("icons/play-small.svg").size(px(11.)))
                            .child("Run")
                            .child(query_keycap("⌘⏎", true))
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.start_query(tab_id, window, cx)
                            })),
                    ),
            )
            .into_any_element()
    }

    fn parameter_header(&self, tab_id: u64, cx: &mut Context<Self>) -> AnyElement {
        div()
            .h(px(38.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .gap_2()
            .px(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(5.))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(FG_SECONDARY)
                    .child(
                        Icon::empty()
                            .path("icons/sparkles.svg")
                            .size(px(11.))
                            .text_color(ACCENT),
                    )
                    .child("Parameters"),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(px(12.))
                    .text_color(FG_MUTED)
                    .child("fill values, then ")
                    .child(query_keycap("⌘⏎", false))
                    .child(" to run"),
            )
            .child(
                div()
                    .id(SharedString::from(format!("close-query-params:{tab_id}")))
                    .tab_index(0)
                    .cursor_pointer()
                    .size(px(22.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(4.))
                    .text_color(FG_MUTED)
                    .hover(|style| style.bg(PANEL_RAISED).text_color(FG))
                    .child(Icon::empty().path("icons/close.svg").size(px(12.)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.query_params.remove(&tab_id);
                        cx.notify();
                    })),
            )
            .into_any_element()
    }

    fn parameter_row(
        &self,
        tab_id: u64,
        index: usize,
        input: &QueryParameterInput,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let kind = input.kind;
        let raw = input.state.read(cx).value().to_string();
        let error = parameter_value(input, cx)
            .err()
            .filter(|_| !raw.is_empty() || kind != ParamKind::Text);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .w(px(200.))
                            .min_w(px(120.))
                            .flex()
                            .items_center()
                            .gap(px(6.))
                            .child(
                                div()
                                    .text_color(ACCENT)
                                    .child(input.parameter.placeholder.clone()),
                            )
                            .when_some(input.parameter.column_hint.clone(), |label, hint| {
                                label.child(
                                    div()
                                        .min_w_0()
                                        .truncate()
                                        .text_size(px(11.))
                                        .text_color(FG_MUTED)
                                        .child(format!("→ {hint}")),
                                )
                            }),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("param-kind:{tab_id}:{index}")))
                            .tab_index(0)
                            .cursor_pointer()
                            .h(px(24.))
                            .w(px(96.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .rounded(px(4.))
                            .border_1()
                            .border_color(BORDER)
                            .bg(INSET)
                            .px(px(7.))
                            .text_color(FG)
                            .child(kind.label())
                            .child(
                                Icon::empty()
                                    .path("icons/chevron-down.svg")
                                    .size(px(9.))
                                    .text_color(FG_MUTED),
                            )
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.cycle_param_kind(tab_id, index, window, cx);
                            })),
                    )
                    .child(self.parameter_value_control(tab_id, index, input, cx)),
            )
            .when_some(error, |row, error| {
                row.child(
                    div()
                        .ml(px(312.))
                        .text_size(px(11.5))
                        .text_color(PROD)
                        .child(error),
                )
            })
            .into_any_element()
    }

    fn parameter_value_control(
        &self,
        tab_id: u64,
        index: usize,
        input: &QueryParameterInput,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match input.kind {
            ParamKind::Null => div()
                .h(px(24.))
                .flex_1()
                .flex()
                .items_center()
                .text_color(FG_MUTED)
                .child("NULL")
                .into_any_element(),
            ParamKind::Boolean => {
                let value = input.state.read(cx).value() == "true";
                div()
                    .id(SharedString::from(format!("param-bool:{tab_id}:{index}")))
                    .tab_index(0)
                    .cursor_pointer()
                    .h(px(24.))
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_between()
                    .rounded(px(4.))
                    .border_1()
                    .border_color(BORDER)
                    .bg(INSET)
                    .px(px(7.))
                    .text_color(FG)
                    .child(if value { "true" } else { "false" })
                    .child(
                        Icon::empty()
                            .path("icons/chevron-down.svg")
                            .size(px(9.))
                            .text_color(FG_MUTED),
                    )
                    .on_click(cx.listener(move |this, _, window, cx| {
                        let Some(input) = this
                            .query_params
                            .get(&tab_id)
                            .and_then(|inputs| inputs.get(index))
                        else {
                            return;
                        };
                        let next = (input.state.read(cx).value() != "true").to_string();
                        input
                            .state
                            .update(cx, |state, cx| state.set_value(next, window, cx));
                    }))
                    .into_any_element()
            }
            _ => div()
                .h(px(24.))
                .flex_1()
                .rounded(px(4.))
                .border_1()
                .border_color(BORDER)
                .bg(INSET)
                .px(px(7.))
                .child(Input::new(&input.state).h_full().appearance(false))
                .into_any_element(),
        }
    }
}
