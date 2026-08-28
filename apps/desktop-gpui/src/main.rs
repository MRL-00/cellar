mod app;
mod app_menu;
mod assets;

use std::{borrow::Cow, sync::Arc};

use app::{preferences::Preferences, sidebar_layout::SidebarLayout, CellarApp, SessionState};
use cellar_desktop_gpui as _;
use cellar_runtime::ConnectionRegistry;
use gpui::{
    point, px, size, App, AppContext, Application, Bounds, TitlebarOptions, WindowBounds,
    WindowOptions,
};
use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
use gpui_component::Root;

fn main() {
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("create Cellar async runtime"),
    );
    let registry = Arc::new(runtime.block_on(ConnectionRegistry::load()));
    let connections = runtime.block_on(registry.list());
    let sidebar_layout = runtime.block_on(SidebarLayout::load(&connections));
    let preferences = runtime.block_on(Preferences::load_classic());
    let restored_session = SessionState::load();

    Application::new()
        .with_assets(assets::Assets)
        .run(move |cx: &mut App| {
            cx.text_system()
                .add_fonts(vec![
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/geist/Geist-Variable.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/geist/Geist-Italic.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/geist/GeistMono-Variable.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/geist/GeistMono-Italic.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/inter/inter-latin-wght-normal.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/inter/inter-latin-ext-wght-normal.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/inter/inter-latin-wght-italic.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/inter/inter-latin-ext-wght-italic.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/roboto/roboto-latin-wght-normal.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/roboto/roboto-latin-ext-wght-normal.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/roboto/roboto-latin-wght-italic.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/roboto/roboto-latin-ext-wght-italic.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/jetbrains-mono/jetbrains-mono-latin-wght-normal.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/jetbrains-mono/jetbrains-mono-latin-ext-wght-normal.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/jetbrains-mono/jetbrains-mono-latin-wght-italic.woff2")),
                    Cow::Borrowed(include_bytes!("../../desktop/src/assets/fonts/jetbrains-mono/jetbrains-mono-latin-ext-wght-italic.woff2")),
                ])
                .expect("load bundled Cellar fonts");
            gpui_component::init(cx);
            LanguageRegistry::singleton().register(
                "sql",
                &LanguageConfig::new(
                    "sql",
                    tree_sitter_sequel::LANGUAGE.into(),
                    Vec::new(),
                    tree_sitter_sequel::HIGHLIGHTS_QUERY,
                    "",
                    "",
                ),
            );
            let registry = Arc::clone(&registry);
            let runtime = Arc::clone(&runtime);
            let default_bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
            let bounds = restored_session
                .as_ref()
                .map(SessionState::window_bounds)
                .unwrap_or(default_bounds);
            let session = restored_session.unwrap_or_else(|| SessionState::empty(bounds));
            cx.open_window(
                WindowOptions {
                    focus: true,
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    window_min_size: Some(size(px(960.), px(600.))),
                    titlebar: Some(TitlebarOptions {
                        title: None,
                        appears_transparent: true,
                        traffic_light_position: Some(point(px(12.), px(9.))),
                    }),
                    ..Default::default()
                },
                move |window, cx| {
                    let app = cx.new(|cx| {
                        CellarApp::new(
                            connections,
                            registry,
                            runtime,
                            bounds,
                            sidebar_layout,
                            preferences,
                            window,
                            cx,
                        )
                    });
                    app.update(cx, |app, cx| {
                        app.restore_session(session, window, cx);
                        app.load_history(cx);
                        app.initialize_ai(cx);
                        app.initialize_updater(cx);
                    });
                    app_menu::setup(&app, cx);
                    cx.new(|cx| Root::new(app, window, cx))
                },
            )
            .expect("open Cellar window");
            cx.activate(true);
        });
}
