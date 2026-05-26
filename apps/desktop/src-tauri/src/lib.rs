pub fn run() {
    tauri::Builder::default()
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running Cellar");
}
