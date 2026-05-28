pub mod commands;
pub mod state;

use state::ConnectionRegistry;

pub fn run() {
    let builder = commands::builder();
    let invoke_handler = builder.invoke_handler();

    let registry = tauri::async_runtime::block_on(ConnectionRegistry::load());

    tauri::Builder::default()
        .manage(registry)
        .invoke_handler(invoke_handler)
        .setup(move |app| {
            builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Cellar");
}
