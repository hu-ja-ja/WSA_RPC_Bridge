#[cfg(not(target_os = "android"))]
mod adb;
#[cfg(not(target_os = "android"))]
mod apk_label;
#[cfg(target_os = "android")]
mod android;
mod artwork;
mod commands;
mod config;
#[cfg(not(target_os = "android"))]
mod discord;
mod i18n;
mod models;

#[cfg(target_os = "windows")]
mod shutdown;
#[cfg(not(target_os = "android"))]
mod tray;

#[cfg(not(target_os = "android"))]
use std::path::PathBuf;

use commands::AppState;
#[cfg(not(target_os = "android"))]
use tauri::{Manager, WindowEvent};
#[cfg(target_os = "android")]
use tauri::Manager;
#[cfg(not(target_os = "android"))]
use tauri_plugin_autostart::ManagerExt;
use tokio::sync::Mutex;

use crate::artwork::ArtworkRegistry;

#[cfg(not(target_os = "android"))]
const DISCORD_CLIENT_ID: &str = "1530562506513449120";
#[cfg(not(target_os = "android"))]
const APP_DIR: &str = "wsa-rpc-bridge";

#[cfg(not(target_os = "android"))]
fn wsa_data_dir() -> PathBuf {
    config::app_data_base("LOCALAPPDATA", "Local").join(APP_DIR)
}

#[cfg(not(target_os = "android"))]
fn default_apk_cache_dir() -> PathBuf {
    wsa_data_dir().join("ApkCache")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(not(target_os = "android"))]
    let apk_cache_dir = default_apk_cache_dir();

    let mut artwork_registry = ArtworkRegistry::new();
    artwork_registry.register(Box::new(artwork::nicobox::NicoboxResolver::new()));

    let builder = tauri::Builder::default();

    #[cfg(not(target_os = "android"))]
    let builder = builder.manage(AppState {
        adb: Mutex::new(adb::AdbClient::new()),
        discord: discord::DiscordRpc::new(DISCORD_CLIENT_ID),
        artwork: Mutex::new(artwork_registry),
        apk_label: Mutex::new(apk_label::ApkLabelResolver::new(apk_cache_dir)),
        config: config::ConfigManager::new(),
    });

    #[cfg(target_os = "android")]
    let builder = builder.manage(AppState {
        artwork: Mutex::new(artwork_registry),
        config: config::ConfigManager::new(),
        discord_connected: std::sync::atomic::AtomicBool::new(false),
    });

    let builder = builder
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
        ]);

    #[cfg(not(target_os = "android"))]
    let builder = builder.plugin(tauri_plugin_autostart::Builder::new().build());

    #[cfg(not(target_os = "android"))]
    let builder = builder.on_window_event(|window, event| {
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
    });

    #[cfg(not(target_os = "android"))]
    let builder = builder.setup(|app| {
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

        #[cfg(target_os = "windows")]
        if let Some(window) = app.get_webview_window("main") {
            shutdown::install(app.handle(), &window);
        }

        let state = app.state::<AppState>();
        let cfg = state.config.get();
        if cfg.auto_start {
            let _ = app.autolaunch().enable();
        }
        if !cfg.start_in_tray {
            tray::show_main_window(app.handle());
        }

        Ok(())
    });

    #[cfg(target_os = "android")]
    let builder = builder.setup(|app| {
        if cfg!(debug_assertions) {
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;
        }
        log::info!("android: app started");

        // フォアグラウンドサービスがプロセスを生かしている間、WebView のポーリングが
        // 止まっても Discord プレゼンスを更新し続ける保険。
        let app_handle = app.handle().clone();
        std::thread::spawn(move || {
            let mut last_title = String::new();
            loop {
                std::thread::sleep(std::time::Duration::from_secs(10));
                let state = app_handle.state::<AppState>();
                if !state
                    .discord_connected
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    continue;
                }
                let info = crate::android::media_state()
                    .lock()
                    .expect("media mutex poisoned")
                    .clone();
                if info.title.is_empty() || info.title == last_title {
                    continue;
                }
                last_title = info.title.clone();
                log::info!(
                    "android: background presence update: {} - {}",
                    info.title,
                    info.artist
                );
                if let Err(e) = crate::android::discord_update_presence(&info) {
                    log::warn!("android: background presence update failed: {e}");
                }
            }
        });

        Ok(())
    });

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
