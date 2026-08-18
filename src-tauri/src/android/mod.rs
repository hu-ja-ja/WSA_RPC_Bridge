pub mod discord;
pub mod media;
pub mod notifications;
pub mod signature;

pub use discord::{
    discord_connect, discord_disconnect, discord_idle_disconnect, discord_update_presence,
    rpc_idle,
};
pub use media::{
    list_media_apps, load_media_notification_enabled, load_rpc_enabled, load_whitelist, media_state,
    save_whitelist, set_app_handle, set_media_notification_enabled, set_rpc_enabled,
};
pub use notifications::{get_notification_access_status, open_notification_access_settings};
pub use signature::get_signing_fingerprint;