mod adb;
mod apk_label;
mod artwork;
mod commands;
mod config;
mod discord;
mod i18n;
mod models;
mod tray;

use std::path::PathBuf;

use commands::AppState;
use tauri::{Manager, WindowEvent};
use tauri_plugin_autostart::ManagerExt;
use tokio::sync::Mutex;

use crate::apk_label::ApkLabelResolver;
use crate::artwork::ArtworkRegistry;

const DISCORD_CLIENT_ID: &str = "1530562506513449120";
const APP_DIR: &str = "wsa-rpc-bridge";

fn wsa_data_dir() -> PathBuf {
    config::app_data_base("LOCALAPPDATA", "Local").join(APP_DIR)
}

fn default_apk_cache_dir() -> PathBuf {
    wsa_data_dir().join("ApkCache")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let apk_cache_dir = default_apk_cache_dir();
    let mut artwork_registry = ArtworkRegistry::new();
    artwork_registry.register(Box::new(artwork::nicobox::NicoboxResolver::new()));

    tauri::Builder::default()
        .manage(AppState {
            adb: Mutex::new(adb::AdbClient::new()),
            discord: discord::DiscordRpc::new(DISCORD_CLIENT_ID),
            artwork: Mutex::new(artwork_registry),
            apk_label: Mutex::new(ApkLabelResolver::new(apk_cache_dir)),
            config: config::ConfigManager::new(),
        })
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_adb_status,
            commands::get_media_info,
            commands::connect_discord,
            commands::disconnect_discord,
            commands::update_discord_presence,
            commands::get_discord_status,
            commands::get_settings,
            commands::update_settings,
        ])
        .on_window_event(|window, event| {
            let app = window.app_handle();
            let state = app.state::<AppState>();
            let cfg = state.config.get();

            if let WindowEvent::Resized(_) = event {
                if cfg.minimize_to_tray && window.is_minimized().unwrap_or(false) {
                    let _ = window.hide();
                }
            }

            if let WindowEvent::CloseRequested { api, .. } = event {
                if cfg.close_to_tray {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;

            std::fs::create_dir_all(wsa_data_dir())?;
            std::fs::create_dir_all(default_apk_cache_dir())?;
            log::info!("data directories created");

            tray::setup_tray(app.handle())?;

            let state = app.state::<AppState>();
            let cfg = state.config.get();
            if cfg.auto_start {
                let _ = app.autolaunch().enable();
            }
            if !cfg.start_in_tray {
                tray::show_main_window(app.handle());
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
