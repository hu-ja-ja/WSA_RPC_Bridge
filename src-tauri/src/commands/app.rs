use tauri::State;
use tokio::sync::Mutex;

use crate::adb::AdbClient;
use crate::apk_label::ApkLabelResolver;
use crate::artwork::ArtworkRegistry;
use crate::config::{AppConfig, ConfigManager};
use crate::discord::DiscordRpc;
use crate::models::MediaInfo;

pub struct AppState {
    pub adb: Mutex<AdbClient>,
    pub discord: DiscordRpc,
    pub artwork: Mutex<ArtworkRegistry>,
    pub apk_label: Mutex<ApkLabelResolver>,
    pub config: ConfigManager,
}

#[tauri::command]
pub async fn get_adb_status(state: State<'_, AppState>) -> Result<bool, String> {
    let connected = state.adb.lock().await.is_connected();
    log::debug!("get_adb_status: adb_connected={}", connected);
    Ok(connected)
}

#[tauri::command]
pub async fn get_media_info(state: State<'_, AppState>) -> Result<MediaInfo, String> {
    log::info!("get_media_info: invoked by frontend");
    let mut adb = state.adb.lock().await;
    let mut result = adb.get_media_info().await;
    drop(adb);

    if let Ok(ref mut info) = result {
        let display_name = state.apk_label.lock().await.resolve(&info.package_name).await;
        info.display_name = Some(display_name);

        let mut registry = state.artwork.lock().await;
        let thumb = registry.resolve(info).await;
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
pub fn connect_discord(state: State<'_, AppState>) -> Result<(), String> {
    log::info!("connect_discord: connecting to Discord IPC");
    state.discord.connect();
    Ok(())
}

#[tauri::command]
pub fn disconnect_discord(state: State<'_, AppState>) -> Result<(), String> {
    log::info!("disconnect_discord: disconnecting from Discord IPC");
    state.discord.disconnect();
    Ok(())
}

#[tauri::command]
pub fn update_discord_presence(state: State<'_, AppState>, info: MediaInfo) -> Result<(), String> {
    log::debug!("update_discord_presence: title={:?}", info.title);
    state.discord.update_presence(&info);
    Ok(())
}

#[tauri::command]
pub fn get_discord_status(state: State<'_, AppState>) -> Result<bool, String> {
    let connected = state.discord.is_connected();
    log::debug!("get_discord_status: discord_connected={}", connected);
    Ok(connected)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<AppConfig, String> {
    Ok(state.config.get())
}

#[tauri::command]
pub fn update_settings(state: State<'_, AppState>, config: AppConfig) -> Result<(), String> {
    state.config.set(config);
    log::info!("update_settings: settings updated");
    Ok(())
}
