use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Spawn in-process FileDeck Rust Core Backend
            tokio::spawn(async move {
                if let Err(e) = filedeck_backend::server::start_server().await {
                    eprintln!("[FileDeck Core Error] Failed to start backend server: {e}");
                }
            });

            // If on main window close, trigger full app exit
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { .. } = event {
                        std::process::exit(0);
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running FileDeck desktop application");
}
