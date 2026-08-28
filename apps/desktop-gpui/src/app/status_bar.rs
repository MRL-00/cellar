use cellar_core::driver::SslMode;
use gpui::{div, point, prelude::*, AnyElement, BoxShadow};
use gpui_component::Icon;

use super::{
    shell_widgets::{engine_color, short_driver_version},
    CellarApp,
};
use cellar_desktop_gpui::{
    model::{ConnectionState, TabKind},
    theme::{
        ui_px, ACCENT, BORDER, FG_DISABLED, FG_MUTED, FG_SECONDARY, FG_TERTIARY, INSERT,
        INSERT_SOFT, PANEL,
    },
};

impl CellarApp {
    pub(super) fn status_bar(&self) -> AnyElement {
        let connection_id = self.model.active_tab().map(|tab| match &tab.kind {
            TabKind::Table { target, .. } => target.connection_id.as_str(),
            TabKind::Query { target, .. } => target.connection_id.as_str(),
            TabKind::ErDiagram { target, .. } => target.connection_id.as_str(),
            TabKind::SchemaCompare { config, .. } => config
                .source
                .live_connection_id()
                .or_else(|| config.target.live_connection_id())
                .unwrap_or_default(),
        });
        let active = connection_id.and_then(|id| {
            self.model
                .connections()
                .iter()
                .find(|config| config.id == id)
        });
        let connected = active.is_some_and(|config| {
            matches!(
                self.model.connection_state(&config.id),
                ConnectionState::Connected
            )
        });
        div()
            .h(ui_px(22.))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(BORDER)
            .bg(PANEL)
            .px(ui_px(10.))
            .text_size(ui_px(11.5))
            .text_color(FG_TERTIARY)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui_px(14.))
                    .child(connection_status(self, active, connected))
                    .when_some(active, |left, config| {
                        left.child(
                            status_item(
                                "icons/user.svg",
                                format!("{}@{}", config.user, config.host),
                            )
                            .font_family(cellar_desktop_gpui::theme::mono_font()),
                        )
                        .when(
                            config.ssl_mode != SslMode::Disable,
                            |left| {
                                left.child(status_item(
                                    "icons/lock.svg",
                                    format!("SSL · {}", ssl_label(config.ssl_mode)),
                                ))
                            },
                        )
                    }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(ui_px(14.))
                    .child(match self.last_query_metrics {
                        Some((rows, truncated, duration_ms)) => div()
                            .h(ui_px(18.))
                            .flex()
                            .items_center()
                            .gap(ui_px(5.))
                            .child(
                                Icon::empty()
                                    .path("icons/check.svg")
                                    .size(ui_px(10.))
                                    .text_color(ACCENT),
                            )
                            .child(
                                div()
                                    .font_family(cellar_desktop_gpui::theme::mono_font())
                                    .child(format!(
                                        "{rows}{} rows · {duration_ms} ms",
                                        if truncated { "+" } else { "" }
                                    )),
                            )
                            .into_any_element(),
                        None => div()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .text_color(FG_MUTED)
                            .child("— rows · — ms")
                            .into_any_element(),
                    })
                    .child(
                        div()
                            .h(ui_px(18.))
                            .flex()
                            .items_center()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .child("UTF-8 · LF"),
                    ),
            )
            .into_any_element()
    }
}

fn connection_status(
    app: &CellarApp,
    active: Option<&cellar_core::driver::ConnectionConfig>,
    connected: bool,
) -> AnyElement {
    div()
        .h(ui_px(18.))
        .flex()
        .items_center()
        .gap(ui_px(5.))
        .child(
            div()
                .size(ui_px(6.))
                .rounded(ui_px(3.))
                .bg(if connected { INSERT } else { FG_DISABLED })
                .when(connected, |dot| {
                    dot.shadow(vec![BoxShadow {
                        color: INSERT_SOFT.rgba().into(),
                        offset: point(ui_px(0.), ui_px(0.)),
                        blur_radius: ui_px(0.),
                        spread_radius: ui_px(2.),
                    }])
                }),
        )
        .child(
            div().text_color(FG_SECONDARY).child(
                active
                    .map_or("no connection", |config| config.name.as_str())
                    .to_owned(),
            ),
        )
        .when_some(active, |item, config| {
            item.child(div().text_color(FG_MUTED).child("·"))
                .child(
                    div()
                        .text_color(engine_color(config.engine))
                        .child(format!("{:?}", config.engine).to_ascii_uppercase()),
                )
                .when_some(app.driver_infos.get(&config.id), |item, info| {
                    item.child(div().text_color(FG_MUTED).child("·")).child(
                        div()
                            .font_family(cellar_desktop_gpui::theme::mono_font())
                            .child(short_driver_version(&info.version)),
                    )
                })
        })
        .into_any_element()
}

fn status_item(icon: &'static str, label: String) -> gpui::Div {
    div()
        .h(ui_px(18.))
        .flex()
        .items_center()
        .gap(ui_px(5.))
        .child(Icon::empty().path(icon).size(ui_px(10.)))
        .child(label)
}

fn ssl_label(mode: SslMode) -> &'static str {
    match mode {
        SslMode::Disable => "disable",
        SslMode::Prefer => "prefer",
        SslMode::Require => "require",
        SslMode::VerifyCa => "verify-ca",
        SslMode::VerifyFull => "verify-full",
    }
}

#[cfg(test)]
mod tests {
    use super::ssl_label;
    use cellar_core::driver::SslMode;

    #[test]
    fn ssl_labels_match_serialized_classic_values() {
        assert_eq!(ssl_label(SslMode::VerifyCa), "verify-ca");
        assert_eq!(ssl_label(SslMode::VerifyFull), "verify-full");
    }
}
