use gpui::{actions, App, Entity, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType, Window};
use gpui_component::input::{Copy, Cut, Paste, Redo, SelectAll, Undo};

use crate::app::CellarApp;

actions!(
    cellar_menu,
    [
        About,
        Settings,
        Hide,
        HideOthers,
        ShowAll,
        Quit,
        Minimize,
        Zoom,
        BringAllToFront,
        NewConnection,
        NewQuery,
        CloseTab,
        ToggleSidebar,
        ToggleAiPanel,
        ToggleBottomPanel,
        ToggleCommandPalette,
        RunQuery,
        RunQueryAll,
        CancelQuery,
        ReviewChanges,
        Find,
        OpenGitHub,
        OpenChangelog
    ]
);

pub fn setup(app: &Entity<CellarApp>, cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("cmd-,", Settings, None),
        KeyBinding::new("cmd-h", Hide, None),
        KeyBinding::new("cmd-alt-h", HideOthers, None),
        KeyBinding::new("cmd-m", Minimize, None),
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-n", NewConnection, None),
        KeyBinding::new("cmd-t", NewQuery, None),
        KeyBinding::new("cmd-k", ToggleCommandPalette, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-b", ToggleSidebar, None),
        KeyBinding::new("cmd-j", ToggleAiPanel, None),
        KeyBinding::new("cmd-down", ToggleBottomPanel, None),
        KeyBinding::new("cmd-enter", RunQuery, None),
        KeyBinding::new("cmd-shift-enter", RunQueryAll, None),
        KeyBinding::new("cmd-.", CancelQuery, None),
        KeyBinding::new("cmd-s", ReviewChanges, None),
        KeyBinding::new("cmd-f", Find, None),
    ]);

    on_app_action::<Settings>(app, cx, |app, cx| app.open_appearance_settings(cx));
    on_app_action::<About>(app, cx, |app, cx| app.open_about_settings(cx));
    on_app_action::<CloseTab>(app, cx, |app, cx| app.close_active_tab(cx));
    on_app_action::<ToggleSidebar>(app, cx, |app, cx| app.toggle_sidebar(cx));
    on_app_action::<ToggleAiPanel>(app, cx, |app, cx| app.toggle_ai_panel(cx));
    on_app_action::<ToggleBottomPanel>(app, cx, |app, cx| app.toggle_bottom_panel(cx));
    on_app_action::<CancelQuery>(app, cx, |app, cx| app.cancel_active_query(cx));
    on_app_action::<ReviewChanges>(app, cx, |app, cx| app.review_pending_changes(cx));

    on_window_action::<ToggleCommandPalette>(app, cx, |app, window, cx| {
        app.toggle_command_palette(window, cx)
    });
    on_window_action::<NewConnection>(app, cx, |app, window, cx| {
        app.open_connection_editor(None, window, cx)
    });
    on_window_action::<NewQuery>(app, cx, |app, window, cx| app.new_query(window, cx));
    on_window_action::<RunQuery>(app, cx, |app, window, cx| app.run_active_query(window, cx));
    on_window_action::<RunQueryAll>(app, cx, |app, window, cx| {
        app.run_active_query_all(window, cx)
    });
    on_window_action::<Find>(app, cx, |app, window, cx| app.focus_find(window, cx));

    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &BringAllToFront, cx| cx.activate(true));
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &OpenGitHub, cx| cx.open_url("https://github.com/MRL-00/cellar"));
    cx.on_action(|_: &OpenChangelog, cx| cx.open_url("https://github.com/MRL-00/cellar/releases"));

    cx.set_menus(vec![
        Menu {
            name: "Cellar".into(),
            items: vec![
                MenuItem::action("About Cellar", About),
                MenuItem::separator(),
                MenuItem::action("Settings…", Settings),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Hide Cellar", Hide),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quit Cellar", Quit),
            ],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New Connection", NewConnection),
                MenuItem::action("New Query", NewQuery),
                MenuItem::separator(),
                MenuItem::action("Close Tab", CloseTab),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, OsAction::Cut),
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
                MenuItem::separator(),
                MenuItem::action("Find", Find),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Command Palette", ToggleCommandPalette),
                MenuItem::separator(),
                MenuItem::action("Toggle Sidebar", ToggleSidebar),
                MenuItem::action("Toggle AI Panel", ToggleAiPanel),
                MenuItem::action("Toggle Output Panel", ToggleBottomPanel),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![
                MenuItem::action("Minimize", Minimize),
                MenuItem::action("Zoom", Zoom),
                MenuItem::separator(),
                MenuItem::action("Bring All to Front", BringAllToFront),
            ],
        },
        Menu {
            name: "Help".into(),
            items: vec![
                MenuItem::action("GitHub", OpenGitHub),
                MenuItem::action("Changelog", OpenChangelog),
            ],
        },
    ]);
}

fn on_app_action<A: gpui::Action>(
    app: &Entity<CellarApp>,
    cx: &mut App,
    listener: impl Fn(&mut CellarApp, &mut gpui::Context<CellarApp>) + 'static,
) {
    let app = app.clone();
    cx.on_action(move |_: &A, cx| {
        app.update(cx, |app, cx| listener(app, cx));
    });
}

fn on_window_action<A: gpui::Action>(
    app: &Entity<CellarApp>,
    cx: &mut App,
    listener: impl Fn(&mut CellarApp, &mut Window, &mut gpui::Context<CellarApp>) + Clone + 'static,
) {
    let app = app.clone();
    cx.on_action(move |_: &A, cx| {
        let Some(handle) = cx.active_window() else {
            return;
        };
        let app = app.clone();
        let listener = listener.clone();
        cx.defer(move |cx| {
            handle
                .update(cx, |_, window, cx| {
                    app.update(cx, |app, cx| listener(app, window, cx));
                })
                .ok();
        });
    });
}
