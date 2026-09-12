#[cfg(target_os = "android")]
use tauri::Emitter;
#[cfg(target_os = "android")]
use tauri::Manager;
use tauri::{AppHandle, State};
#[cfg(not(target_os = "android"))]
use tauri_plugin_autostart::ManagerExt;
use tokio::sync::Mutex;

#[cfg(not(target_os = "android"))]
use crate::adb::AdbClient;
#[cfg(not(target_os = "android"))]
use crate::apk_label::ApkLabelResolver;
use crate::artwork::ArtworkRegistry;
use crate::config::{AppConfig, ConfigManager};
#[cfg(not(target_os = "android"))]
use crate::discord::DiscordRpc;
use crate::models::MediaInfo;

pub struct AppState {
    #[cfg(not(target_os = "android"))]
    pub adb: Mutex<AdbClient>,
    #[cfg(not(target_os = "android"))]
    pub discord: DiscordRpc,
    pub artwork: Mutex<ArtworkRegistry>,
    #[cfg(not(target_os = "android"))]
    pub apk_label: Mutex<ApkLabelResolver>,
    pub config: ConfigManager,
    #[cfg(target_os = "android")]
    pub discord_connected: std::sync::atomic::AtomicBool,
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub async fn get_adb_status(state: State<'_, AppState>) -> Result<bool, String> {
    let connected = state.adb.lock().await.is_connected();
    log::debug!("get_adb_status: adb_connected={}", connected);
    Ok(connected)
}

#[tauri::command]
#[cfg(target_os = "android")]
pub async fn get_adb_status(_state: State<'_, AppState>) -> Result<bool, String> {
    Ok(false)
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub async fn get_media_info(state: State<'_, AppState>) -> Result<MediaInfo, String> {
    log::debug!("get_media_info: invoked by frontend");
    // ponytail: dumpsysのみADB確保。ラベル解決後は解放しYouTube検索を並行可に
    let mut result = {
        let mut adb = state.adb.lock().await;
        adb.get_media_info().await
    };

    if let Ok(ref mut info) = result {
        // ponytail: キャッシュ命中はADB不要。5秒ポーリング詰まり防止
        let cached = state.apk_label.lock().await.cached(&info.package_name);
        let display_name = match cached {
            Some(name) => name,
            None => {
                let mut adb = state.adb.lock().await;
                match adb.device() {
                    Some(device) => {
                        state
                            .apk_label
                            .lock()
                            .await
                            .resolve(&info.package_name, device)
                            .await
                    }
                    None => info.package_name.clone(),
                }
            }
        };
        info.display_name = Some(display_name);

        let thumb = {
            let mut registry = state.artwork.lock().await;
            registry.resolve(info).await
        };
        if let Some(ref url) = thumb {
            info.thumbnail_url = Some(url.clone());
        }

        log::info!(
            "get_media_info: success title={:?}, artist={:?}, package={}, thumbnail={}",
            info.title,
            info.artist,
            info.package_name,
            if thumb.is_some() { "有" } else { "無" },
        );
    } else if let Err(e) = &result {
        log::error!("get_media_info: failed: {e:#}");
    }

    result.map_err(|e| e.to_string())
}

#[tauri::command]
#[cfg(target_os = "android")]
pub async fn get_media_info(
    app: AppHandle,
    _state: State<'_, AppState>,
) -> Result<MediaInfo, String> {
    let info = crate::android::media_state()
        .lock()
        .expect("media mutex poisoned")
        .clone();

    log::info!(
        "get_media_info: android title={:?}, artist={:?}",
        info.title,
        info.artist
    );

    // ponytail: 情報は即時返し、サムネは別イベントで後送 — 秒数リセットを防ぐ
    if !info.title.is_empty() {
        let app_clone = app.clone();
        let info_clone = info.clone();
        tauri::async_runtime::spawn(async move {
            let state = app_clone.state::<AppState>();
            let url_opt = state.artwork.lock().await.resolve(&info_clone).await;
            if let Some(url) = url_opt {
                // stale check
                let cur = crate::android::media_state()
                    .lock()
                    .expect("media mutex poisoned")
                    .clone();
                if cur.package_name != info_clone.package_name
                    || cur.title != info_clone.title
                    || cur.artist != info_clone.artist
                {
                    return;
                }
                #[derive(serde::Serialize, Clone)]
                struct Payload {
                    package_name: String,
                    title: String,
                    artist: String,
                    thumbnail_url: String,
                }
                let payload = Payload {
                    package_name: info_clone.package_name,
                    title: info_clone.title,
                    artist: info_clone.artist,
                    thumbnail_url: url,
                };
                let _ = app_clone.emit("thumbnail-updated", &payload);
            }
        });
    }

    Ok(info)
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub async fn connect_discord(state: State<'_, AppState>) -> Result<(), String> {
    log::debug!("connect_discord: connecting to Discord IPC");
    state.discord.connect()
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn connect_discord(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    if crate::android::rpc_idle() {
        return Ok(());
    }
    log::info!("connect_discord: connecting Discord RPC (android)");
    crate::android::discord_connect()?;
    state
        .discord_connected
        .store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = app.emit("discord-status-changed", true);
    Ok(())
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub async fn disconnect_discord(state: State<'_, AppState>) -> Result<(), String> {
    log::debug!("disconnect_discord: disconnecting from Discord IPC");
    state.discord.disconnect()
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn disconnect_discord(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    log::info!("disconnect_discord: disconnecting Discord RPC (android)");
    crate::android::discord_disconnect()?;
    state
        .discord_connected
        .store(false, std::sync::atomic::Ordering::Relaxed);
    let _ = app.emit("discord-status-changed", false);
    Ok(())
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub async fn update_discord_presence(
    state: State<'_, AppState>,
    info: MediaInfo,
) -> Result<(), String> {
    log::debug!("update_discord_presence: title={:?}", info.title);
    state.discord.update_presence(&info)
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn update_discord_presence(_state: State<'_, AppState>, info: MediaInfo) -> Result<(), String> {
    log::debug!("update_discord_presence: title={:?}", info.title);
    crate::android::discord_update_presence(&info)
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub fn get_discord_status(state: State<'_, AppState>) -> Result<bool, String> {
    let connected = state.discord.is_connected();
    log::debug!("get_discord_status: discord_connected={}", connected);
    Ok(connected)
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn get_discord_status(state: State<'_, AppState>) -> Result<bool, String> {
    let connected = state
        .discord_connected
        .load(std::sync::atomic::Ordering::Relaxed);
    log::debug!(
        "get_discord_status: android discord_connected={}",
        connected
    );
    Ok(connected)
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn get_rpc_enabled() -> Result<bool, String> {
    crate::android::load_rpc_enabled()
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn set_rpc_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    crate::android::set_rpc_enabled(&app, enabled)
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub fn list_media_apps() -> Result<Vec<String>, String> {
    Ok(vec![])
}

#[tauri::command]
#[cfg(target_os = "android")]
pub async fn list_media_apps() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(crate::android::list_media_apps)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.config.get())
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppConfig, String> {
    let mut config = state.config.get();
    config.media_whitelist = crate::android::load_whitelist()?;
    config.media_notification = crate::android::load_media_notification_enabled()?;
    Ok(config)
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub fn update_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    config: AppConfig,
) -> Result<(), String> {
    let old = state.config.get();
    state.config.set(config.clone())?;
    if old.auto_start != config.auto_start {
        if config.auto_start {
            let _ = app.autolaunch().enable();
            log::info!("update_settings: auto-start enabled");
        } else {
            let _ = app.autolaunch().disable();
            log::info!("update_settings: auto-start disabled");
        }
    }
    log::info!("update_settings: settings updated");
    Ok(())
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn update_settings(
    _app: AppHandle,
    state: State<'_, AppState>,
    config: AppConfig,
) -> Result<(), String> {
    crate::android::save_whitelist(&config.media_whitelist)?;
    crate::android::set_media_notification_enabled(config.media_notification)?;
    state.config.set(config.clone())?;
    log::info!("update_settings: settings updated");
    Ok(())
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub fn get_notification_access_status() -> Result<bool, String> {
    Ok(true)
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn get_notification_access_status() -> Result<bool, String> {
    crate::android::get_notification_access_status()
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub fn open_notification_access_settings() -> Result<(), String> {
    Ok(())
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn open_notification_access_settings() -> Result<(), String> {
    crate::android::open_notification_access_settings()
}

#[tauri::command]
#[cfg(target_os = "android")]
pub async fn check_android_update(
    app: AppHandle,
) -> Result<crate::android::AndroidUpdateStatus, String> {
    let current = app.package_info().version.to_string();
    crate::android::fetch_update_status(&current).await
}

#[tauri::command]
#[cfg(target_os = "android")]
pub async fn download_android_update(
    app: AppHandle,
    url: String,
    version: String,
) -> Result<String, String> {
    crate::android::download_update(&app, &url, &version).await
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn install_android_update(path: String) -> Result<(), String> {
    crate::android::install_apk(&path)
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn get_install_permission_status() -> Result<bool, String> {
    crate::android::can_install_packages()
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn open_install_permission_settings() -> Result<(), String> {
    crate::android::open_install_settings()
}

#[tauri::command]
#[cfg(not(target_os = "android"))]
pub fn get_signing_fingerprint() -> Result<Option<String>, String> {
    Ok(None)
}

#[tauri::command]
#[cfg(target_os = "android")]
pub fn get_signing_fingerprint() -> Result<Option<String>, String> {
    let fp = crate::android::get_signing_fingerprint()?;
    Ok((!fp.is_empty()).then_some(fp))
}
