pub mod discord;
pub mod media;

pub use discord::{
    discord_connect, discord_disconnect, discord_idle_disconnect, discord_update_presence,
    rpc_idle,
};
pub use media::{list_media_apps, load_whitelist, media_state, save_whitelist, set_app_handle};