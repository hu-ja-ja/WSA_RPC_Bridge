use tauri::State;
use tokio::sync::Mutex;

use crate::adb::AdbClient;
use crate::discord::DiscordRpc;
use crate::models::MediaInfo;

pub struct AppState {
    pub adb: Mutex<AdbClient>,
    pub discord: DiscordRpc,
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
    let result = adb.get_media_info().await;
    match &result {
        Ok(info) => log::info!(
            "get_media_info: success title={:?}, artist={:?}",
            info.title,
            info.artist,
        ),
        Err(e) => log::error!("get_media_info: failed: {e:#}"),
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
