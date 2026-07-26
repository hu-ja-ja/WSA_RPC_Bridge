mod adb;
mod apk_label;
mod artwork;
mod commands;
mod config;
mod discord;
mod models;

use std::path::PathBuf;

use commands::AppState;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, WindowEvent,
};
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
            config: config::ConfigManager::new(),
        })
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

            let open = MenuItem::with_id(app, "open", "開く", true, None::<&str>)?;
            let settings = MenuItem::with_id(app, "settings", "設定", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &settings, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .show_menu_on_left_click(false)
                .menu(&menu)
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "open" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                                let _ = window.emit("show-settings", ());
                            }
                        }
                        "quit" => {
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            let state = app.state::<AppState>();
            let cfg = state.config.get();
            if cfg.start_in_tray {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
