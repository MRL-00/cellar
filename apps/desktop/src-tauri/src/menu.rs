//! Native application menu.
//!
//! macOS renders a system menu bar; everywhere else Cellar drives its chrome
//! from the custom React title bar. So on macOS we take Tauri's default menu
//! (which already wires up About, Services, Hide/Quit, Edit, View, and Window)
//! and only add a "Settings…" item to the app menu. Selecting it emits
//! [`SETTINGS_EVENT`] to the frontend, which opens the same settings modal as
//! the in-app `⌘,` shortcut and the sidebar gear.

/// Event emitted to the frontend when the app-menu "Settings…" item is chosen.
/// The frontend listens for this and opens the settings modal.
#[cfg(target_os = "macos")]
pub const SETTINGS_EVENT: &str = "menu://settings";

/// Install the native macOS menu with a "Settings…" item wired to
/// [`SETTINGS_EVENT`]. No-op on other platforms.
#[cfg(target_os = "macos")]
pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItemBuilder, PredefinedMenuItem};
    use tauri::Emitter;

    let menu = Menu::default(app)?;

    let settings = MenuItemBuilder::new("Settings…")
        .id("settings")
        .accelerator("Cmd+,")
        .build(app)?;

    // The first submenu is the app ("Cellar") menu. Its default layout starts
    // `About / ─── / Services / …`; slot `Settings… / ───` in after the first
    // separator, the conventional macOS location for Settings.
    let items = menu.items()?;
    if let Some(app_menu) = items.first().and_then(|item| item.as_submenu()) {
        app_menu.insert(&settings, 2)?;
        app_menu.insert(&PredefinedMenuItem::separator(app)?, 3)?;
    }

    app.set_menu(menu)?;
    app.on_menu_event(move |app, event| {
        if event.id() == "settings" {
            let _ = app.emit(SETTINGS_EVENT, ());
        }
    });

    Ok(())
}

/// No native menu on non-macOS platforms; the React title bar owns the chrome.
#[cfg(not(target_os = "macos"))]
pub fn setup(_app: &tauri::AppHandle) -> tauri::Result<()> {
    Ok(())
}
