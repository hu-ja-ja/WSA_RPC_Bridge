use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Instant;

use adb_client::{server::ADBServer, server_device::ADBServerDevice, ADBDeviceExt};
use anyhow::{Context, Result};

use super::parser::parse_media_session;
use crate::models::MediaInfo;

const WSA_PORT: u16 = 58526;
const ADB_SERVER_PORT: u16 = 5037;
const DEBUG_TRUNCATE_LEN: usize = 2000;

pub struct AdbClient {
    device: Option<ADBServerDevice>,
    connected: bool,
}

impl Drop for AdbClient {
    fn drop(&mut self) {
        if self.connected {
            log::info!("ADB: client dropped (was connected)");
        }
    }
}

impl AdbClient {
    pub fn new() -> Self {
        log::debug!("AdbClient::new()");
        Self {
            device: None,
            connected: false,
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub async fn connect(&mut self) -> Result<()> {
        let server_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, ADB_SERVER_PORT);
        let ws_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), WSA_PORT);

        log::info!("ADB connect: server={server_addr}, target={ws_addr}");
        let start = Instant::now();

        let mut server = ADBServer::new(server_addr);
        match server.connect_device(ws_addr) {
            Ok(()) => log::info!("ADB: connected to WSA device"),
            Err(e) => {
                log::warn!("ADB connect_device (ignored, may already be connected): {e:#}");
            }
        }
        drop(server);

        let serial = format!("127.0.0.1:{WSA_PORT}");
        let mut device = ADBServerDevice::new(serial, Some(server_addr));

        let mut buf = Vec::new();
        device
            .shell_command(&"echo adb_ok", Some(&mut buf), None)
            .context("ADB device not reachable. Try `adb connect 127.0.0.1:58526` manually.")?;

        let response = String::from_utf8_lossy(&buf);
        let trimmed = response.trim();
        log::info!("ADB echo test: {trimmed:?} (elapsed: {:?})", start.elapsed());

        if trimmed != "adb_ok" {
            log::warn!("ADB echo response unexpected: {trimmed:?}");
        }

        self.device = Some(device);
        self.connected = true;
        log::debug!("ADB connected successfully");
        Ok(())
    }

    pub async fn get_media_info(&mut self) -> Result<MediaInfo> {
        if !self.connected {
            log::info!("ADB: reconnecting before get_media_info");
            self.connect().await?;
        }

        let device = self
            .device
            .as_mut()
            .context("ADB not connected")?;

        let mut raw = Vec::new();
        let start = Instant::now();
        device
            .shell_command(&"dumpsys media_session", Some(&mut raw), None)
            .context("Failed to execute dumpsys media_session")?;
        let elapsed = start.elapsed();
        log::info!("ADB dumpsys returned {} bytes ({:?})", raw.len(), elapsed);

        let output_str =
            String::from_utf8(raw).context("Failed to decode dumpsys output as UTF-8")?;

        match parse_media_session(&output_str) {
            Some(info) => {
                log::info!(
                    "ADB parsed media: title={:?}, artist={:?}, playing={}",
                    info.title,
                    info.artist,
                    info.is_playing,
                );
                Ok(info)
            }
            None => {
                log::warn!(
                    "ADB no active media session (dumpsys: {} chars)",
                    output_str.len()
                );
                log::debug!("ADB dumpsys output:\n{}", &output_str[..output_str.len().min(DEBUG_TRUNCATE_LEN)]);
                Err(anyhow::anyhow!("No active media session found"))
            }
        }
    }
}
