use gpui::{actions, App, Entity, KeyBinding, Menu, MenuItem, OsAction, SystemMenuType};
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
    ]);

    let settings_app = app.clone();
    cx.on_action(move |_: &Settings, cx| {
        settings_app.update(cx, |app, cx| app.open_appearance_settings(cx));
    });
    let palette_app = app.clone();
    cx.on_action(move |_: &ToggleCommandPalette, cx| {
        let Some(window) = cx.active_window() else {
            return;
        };
        let app = palette_app.clone();
        window
            .update(cx, |_, window, cx| {
                app.update(cx, |app, cx| app.toggle_command_palette(window, cx));
            })
            .ok();
    });
    let about_app = app.clone();
    cx.on_action(move |_: &About, cx| {
        about_app.update(cx, |app, cx| app.open_about_settings(cx));
    });
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx| cx.unhide_other_apps());
    cx.on_action(|_: &BringAllToFront, cx| cx.activate(true));
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.on_action(|_: &OpenGitHub, cx| cx.open_url("https://github.com/MRL-00/cellar"));
    cx.on_action(|_: &OpenChangelog, cx| {
        cx.open_url("https://github.com/MRL-00/cellar/releases")
    });

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
