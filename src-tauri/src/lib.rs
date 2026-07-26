mod adb;
mod apk_label;
mod artwork;
mod commands;
mod discord;
mod models;

use std::path::PathBuf;

use commands::AppState;
use tauri::Manager;
use tokio::sync::Mutex;

use crate::apk_label::ApkLabelResolver;
use crate::artwork::ArtworkRegistry;

const DISCORD_CLIENT_ID: &str = "1530562506513449120";

fn default_cache_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs_data_local()
        });
    base.join("wsa-rpc-bridge").join("Cache")
}

fn default_apk_cache_dir() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs_data_local()
        });
    base.join("wsa-rpc-bridge").join("ApkCache")
}

fn dirs_data_local() -> PathBuf {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("AppData").join("Local").join("wsa-rpc-bridge").join("Cache")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cache_dir = default_cache_dir();
    let apk_cache_dir = default_apk_cache_dir();
    let mut artwork_registry = ArtworkRegistry::new(cache_dir);
    artwork_registry.register(Box::new(artwork::nicobox::NicoboxResolver));

    tauri::Builder::default()
        .manage(AppState {
            adb: Mutex::new(adb::AdbClient::new()),
            discord: discord::DiscordRpc::new(DISCORD_CLIENT_ID),
            artwork: Mutex::new(artwork_registry),
            apk_label: Mutex::new(ApkLabelResolver::new(apk_cache_dir)),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_adb_status,
            commands::get_media_info,
            commands::connect_discord,
            commands::disconnect_discord,
            commands::update_discord_presence,
            commands::get_discord_status,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(2));
                let state = handle.state::<AppState>();
                state.discord.connect();
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
