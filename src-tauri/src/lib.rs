mod adb;
mod commands;
mod discord;
mod models;

use commands::AppState;
use tokio::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            adb: Mutex::new(adb::AdbClient::new()),
            discord: discord::DiscordRpc::new(),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_adb_status,
            commands::get_media_info,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
