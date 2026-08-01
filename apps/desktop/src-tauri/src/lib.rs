pub mod ai_backend;
pub mod commands;
pub mod datagrip;
pub mod history;
pub mod menu;
pub mod openai;
pub mod state;

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
        .manage(ai_backend::BackendAiService::default())
        .manage(openai::OpenAiService::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(invoke_handler)
        .setup(move |app| {
            builder.mount_events(app);
            menu::setup(app.handle())?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Cellar");
}
