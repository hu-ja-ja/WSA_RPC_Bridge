use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};

use discord_rich_presence::{
    activity::{Activity, ActivityType, Timestamps},
    DiscordIpc, DiscordIpcClient,
};

use crate::models::MediaInfo;

enum DiscordCmd {
    Connect,
    UpdatePresence(MediaInfo),
    Disconnect,
}

pub struct DiscordRpc {
    tx: mpsc::Sender<DiscordCmd>,
    connected: Arc<AtomicBool>,
}

impl DiscordRpc {
    pub fn new(client_id: &str) -> Self {
        let cid = client_id.to_string();
        let (tx, rx) = mpsc::channel::<DiscordCmd>();
        let connected = Arc::new(AtomicBool::new(false));
        let connected_clone = connected.clone();

        std::thread::Builder::new()
            .name("discord-ipc".into())
            .spawn(move || {
                let mut client: Option<DiscordIpcClient> = None;
                while let Ok(cmd) = rx.recv() {
                    match cmd {
                        DiscordCmd::Connect => {
                            if client.is_some() {
                                continue;
                            }
                            let mut c = DiscordIpcClient::new(&cid);
                            match c.connect() {
                                Ok(()) => {
                                    log::info!("Discord IPC connected");
                                    client = Some(c);
                                    connected_clone.store(true, Ordering::Relaxed);
                                }
                                Err(e) => {
                                    log::error!("Discord IPC connect failed: {e}");
                                }
                            }
                        }
                        DiscordCmd::UpdatePresence(info) => {
                            let c = match client.as_mut() {
                                Some(c) => c,
                                None => {
                                    log::warn!("Discord not connected, skipping presence update");
                                    continue;
                                }
                            };
                            let mut activity = Activity::new()
                                .name(&info.title)
                                .details(&info.title)
                                .state(&info.artist)
                                .activity_type(ActivityType::Listening);
                            if info.is_playing {
                                if let Some(pos) = info.position {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs() as i64;
                                    let start = now - (pos as i64 / 1000);
                                    let mut ts = Timestamps::new().start(start);
                                    if let Some(dur) = info.duration {
                                        ts = ts.end(start + (dur as i64 / 1000));
                                    }
                                    activity = activity.timestamps(ts);
                                }
                            }
                            if let Err(e) = c.set_activity(activity) {
                                log::error!("Discord set_activity failed: {e}");
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
            })
            .expect("failed to spawn discord-ipc thread");

        Self { tx, connected }
    }

    pub fn connect(&self) {
        let _ = self.tx.send(DiscordCmd::Connect);
    }

    pub fn update_presence(&self, info: &MediaInfo) {
        let _ = self.tx.send(DiscordCmd::UpdatePresence(info.clone()));
    }

    pub fn disconnect(&self) {
        let _ = self.tx.send(DiscordCmd::Disconnect);
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}
