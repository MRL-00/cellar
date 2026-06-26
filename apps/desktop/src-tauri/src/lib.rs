pub mod commands;
pub mod history;
pub mod menu;
pub mod state;

use commands::dump::DumpRegistry;
use history::HistoryStore;
use state::ConnectionRegistry;

pub fn run() {
    let builder = commands::builder();
    let invoke_handler = builder.invoke_handler();

    let registry = tauri::async_runtime::block_on(ConnectionRegistry::load());
    let history = tauri::async_runtime::block_on(HistoryStore::load())
        .unwrap_or_else(|err| HistoryStore::unavailable(err.to_string()));

    tauri::Builder::default()
        .manage(registry)
        .manage(history)
        .manage(DumpRegistry::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(invoke_handler)
        .setup(move |app| {
            builder.mount_events(app);
            menu::setup(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Cellar");
}
