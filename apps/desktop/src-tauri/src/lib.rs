pub mod commands;
pub mod history;
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
        .invoke_handler(invoke_handler)
        .setup(move |app| {
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Cellar");
}
