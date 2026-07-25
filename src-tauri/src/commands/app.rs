use tauri::State;

use crate::adb::AdbClient;
use crate::discord::DiscordRpc;

pub struct AppState {
    pub adb: AdbClient,
    pub discord: DiscordRpc,
}

#[tauri::command]
pub fn get_adb_status(state: State<AppState>) -> Result<bool, String> {
    Ok(state.discord.is_connected())
}
