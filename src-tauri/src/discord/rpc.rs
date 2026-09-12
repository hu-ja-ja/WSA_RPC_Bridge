use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

use discord_rich_presence::{
    activity::{Activity, ActivityType, Assets, Timestamps},
    DiscordIpc, DiscordIpcClient,
};

use crate::models::MediaInfo;

enum DiscordCmd {
    Connect,
    UpdatePresence(MediaInfo),
    Disconnect,
}

// ponytail: 起動時競合に単発で負けないよう数回だけ即時再試行。無限にしない
const CONNECT_ATTEMPTS: u32 = 3;

fn connect_client(cid: &str) -> Option<DiscordIpcClient> {
    for attempt in 1..=CONNECT_ATTEMPTS {
        let mut c = DiscordIpcClient::new(cid);
        match c.connect() {
            Ok(()) => return Some(c),
            Err(e) => {
                if attempt < CONNECT_ATTEMPTS {
                    log::warn!(
                        "Discord IPC connect failed (attempt {attempt}/{CONNECT_ATTEMPTS}): {e}; retrying"
                    );
                    std::thread::sleep(std::time::Duration::from_secs(1));
                } else {
                    log::error!(
                        "Discord IPC connect failed after {CONNECT_ATTEMPTS} attempts: {e}"
                    );
                }
            }
        }
    }
    None
}

pub struct DiscordRpc {
    tx: mpsc::SyncSender<DiscordCmd>,
    connected: Arc<AtomicBool>,
}

impl DiscordRpc {
    pub fn new(client_id: &str) -> Self {
        let cid = client_id.to_string();
        let (tx, rx) = mpsc::sync_channel::<DiscordCmd>(8);
        let connected = Arc::new(AtomicBool::new(false));
        let connected_clone = connected.clone();

        match std::thread::Builder::new()
            .name("discord-ipc".into())
            .spawn(move || {
                let mut client: Option<DiscordIpcClient> = None;
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        DiscordCmd::Connect => {
                            if client.is_some()
                                && connected_clone.load(Ordering::Relaxed)
                            {
                                continue;
                            }
                            // 死んだハンドル温存を避けるため作り直す
                            let _ = client.take();
                            match connect_client(&cid) {
                                Some(c) => {
                                    log::info!("Discord IPC connected");
                                    client = Some(c);
                                    connected_clone.store(true, Ordering::Relaxed);
                                }
                                None => {
                                    connected_clone.store(false, Ordering::Relaxed);
                                }
                            }
                        }
                        DiscordCmd::UpdatePresence(info) => {
                            if client.is_none() {
                                log::info!("Discord not connected, attempting reconnect");
                                match connect_client(&cid) {
                                    Some(c) => {
                                        log::info!("Discord IPC reconnected");
                                        client = Some(c);
                                        connected_clone.store(true, Ordering::Relaxed);
                                    }
                                    None => continue,
                                }
                            }
                            let c = match client.as_mut() {
                                Some(c) => c,
                                None => continue,
                            };
                            let app_name = info
                                .display_name
                                .as_deref()
                                .unwrap_or(&info.package_name);
                            let mut activity = Activity::new()
                                .name(&info.title)
                                .details(app_name)
                                .state(&info.title)
                                .activity_type(ActivityType::Listening);
                            if let Some(ref thumb) = info.thumbnail_url {
                                log::info!("Discord presence: large_image={}", thumb);
                                let assets = Assets::new()
                                    .large_image(thumb.clone())
                                    .large_text(&info.artist);
                                activity = activity.assets(assets);
                            } else {
                                log::debug!("Discord presence: no thumbnail_url");
                            }
                            if info.is_playing {
                                if let Some(pos) = info.position {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or(std::time::Duration::ZERO)
                                        .as_secs() as i64;
                                    let start = now - (pos as i64 / 1000);
                                    let mut ts = Timestamps::new().start(start);
                                    if let Some(dur) = info.duration {
                                        ts = ts.end(start + (dur as i64 / 1000));
                                    }
                                    activity = activity.timestamps(ts);
                                }
                            }
                            if log::log_enabled!(log::Level::Debug) {
                                if let Ok(json) = serde_json::to_string(&activity) {
                                    log::debug!("Discord presence payload: {}", json);
                                }
                            }
                            if let Err(e) = c.set_activity(activity) {
                                log::error!("Discord set_activity failed: {e}; dropping client for reconnect");
                                let _ = client.take();
                                connected_clone.store(false, Ordering::Relaxed);
                            } else {
                                log::info!("Discord presence updated: {} - {}", info.title, info.artist);
                            }
                        }
                        DiscordCmd::Disconnect => {
                            if let Some(mut c) = client.take() {
                                let _ = c.close();
                                log::info!("Discord IPC disconnected");
                            }
                            connected_clone.store(false, Ordering::Relaxed);
                        }
                    }
                }
                if let Some(mut c) = client.take() {
                    let _ = c.close();
                }
                connected_clone.store(false, Ordering::Relaxed);
            }) {
            Ok(_) => log::info!("Discord IPC thread spawned"),
            Err(e) => log::error!("Failed to spawn Discord IPC thread: {e}"),
        }

        Self { tx, connected }
    }

    // ponytail: 同期コマンドはメインスレッド実行のため blocking send 禁止。満杯は Err で返し frontend が次pollで再送する
    pub fn connect(&self) -> Result<(), String> {
        self.tx
            .try_send(DiscordCmd::Connect)
            .map_err(|e| format!("discord queue busy: {e}"))
    }

    pub fn update_presence(&self, info: &MediaInfo) -> Result<(), String> {
        self.tx
            .try_send(DiscordCmd::UpdatePresence(info.clone()))
            .map_err(|e| format!("discord queue busy: {e}"))
    }

    pub fn disconnect(&self) -> Result<(), String> {
        self.tx
            .try_send(DiscordCmd::Disconnect)
            .map_err(|e| format!("discord queue busy: {e}"))
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}
