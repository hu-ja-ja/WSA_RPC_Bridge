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
pub fn get_adb_status(state: State<AppState>) -> Result<bool, String> {
    let connected = state.discord.is_connected();
    log::debug!("get_adb_status: discord_connected={}", connected);
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
