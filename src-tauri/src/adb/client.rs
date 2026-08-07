use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream};
use std::process::Command;
use std::time::{Duration, Instant};

use adb_client::server_device::ADBServerDevice;
use adb_client::ADBDeviceExt;
use anyhow::{Context, Result};

use super::parser::parse_media_session;
use crate::i18n::tr;
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

    pub fn device(&mut self) -> Option<&mut ADBServerDevice> {
        self.device.as_mut()
    }

    pub async fn connect(&mut self) -> Result<()> {
        let server_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, ADB_SERVER_PORT);
        let ws_addr = format!("127.0.0.1:{WSA_PORT}");

        log::info!("ADB connect: server={server_addr}, target={ws_addr}");
        let start = Instant::now();

        // ponytail/known-bug: adb_client's `ADBServer` spawns `adb.exe start-server`
        // on every connection attempt, even when the server is already running
        // (adb_server.rs `connect()` -> `start()` is unconditional). Spawning a
        // process during Windows shutdown races with the loader teardown and
        // produces adb.exe 0xc0000142. If the upstream crate ever fixes this
        // (e.g. by probing the port before spawning), the helpers below can be
        // replaced with plain `ADBServer::new(...)`.
        ensure_adb_server_running(server_addr);

        // Raw host commands instead of ADBServer::disconnect/connect_device, which
        // would respawn adb.exe each time.
        for cmd in [
            format!("host:disconnect:{ws_addr}"),
            format!("host:connect:{ws_addr}"),
        ] {
            match adb_host_command(server_addr, &cmd) {
                Ok(body) => log::info!("adb {} -> {}", cmd, String::from_utf8_lossy(&body).trim()),
                Err(e) => log::debug!("adb {} (ignored): {e:#}", cmd),
            }
        }

        let serial = ws_addr.clone();
        let mut device = ADBServerDevice::new(serial, Some(server_addr));

        let mut buf = Vec::new();
        device
            .shell_command(&"echo adb_ok", Some(&mut buf), None)
            .context(tr("adb.connect_failed"))?;

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

    fn dump_media_session(&mut self) -> Result<Vec<u8>> {
        let device = self
            .device
            .as_mut()
            .context("ADB not connected")?;

        let mut raw = Vec::new();
        let start = Instant::now();
        device
            .shell_command(&"dumpsys media_session", Some(&mut raw), None)?;
        let elapsed = start.elapsed();
        log::info!("ADB dumpsys returned {} bytes ({:?})", raw.len(), elapsed);
        Ok(raw)
    }

    pub async fn get_media_info(&mut self) -> Result<MediaInfo> {
        if !self.connected {
            log::info!("ADB: reconnecting before get_media_info");
            self.connect().await?;
        }

        let raw = match self.dump_media_session() {
            Ok(raw) => raw,
            Err(_) if self.connected => {
                log::warn!("ADB dumpsys failed; forcing reconnect and retrying once");
                self.connected = false;
                self.device = None;
                self.connect().await?;
                self.dump_media_session().context(tr("adb.dumpsys_failed"))?
            }
            Err(e) => return Err(e.context(tr("adb.dumpsys_failed"))),
        };

        let output_str =
            String::from_utf8(raw).context(tr("adb.utf8_decode_failed"))?;

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
                Err(anyhow::anyhow!("{}", tr("adb.no_session")))
            }
        }
    }
}

fn tcp_open(addr: SocketAddrV4) -> bool {
    TcpStream::connect_timeout(&SocketAddr::V4(addr), Duration::from_millis(200)).is_ok()
}

/// Ensure the adb server is running, spawning `adb start-server` only when it
/// is actually down. See the comment in `connect()` about adb_client's
/// unconditional-spawn bug.
fn ensure_adb_server_running(server_addr: SocketAddrV4) {
    if tcp_open(server_addr) {
        return;
    }

    log::info!("ADB: server not running, starting via 'adb start-server'");
    let mut cmd = Command::new("adb");
    cmd.arg("start-server");
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: suppress console windows, same as adb_client does.
        cmd.creation_flags(0x08000000);
    }
    if let Ok(mut child) = cmd.spawn() {
        let _ = child.wait();
    }

    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if tcp_open(server_addr) {
            return;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    log::warn!("ADB server did not start within 5s ('adb' missing from PATH?)");
}

/// Send a raw `host:` command to the adb server and return the response body.
fn adb_host_command(server_addr: SocketAddrV4, cmd: &str) -> Result<Vec<u8>> {
    let mut stream = TcpStream::connect_timeout(&SocketAddr::V4(server_addr), Duration::from_millis(500))
        .with_context(|| format!("adb server unreachable at {server_addr}"))?;
    stream.write_all(format!("{:04x}{cmd}", cmd.len()).as_bytes())?;

    let mut status = [0u8; 4];
    stream.read_exact(&mut status)?;
    let body = read_adb_body(&mut stream)?;

    if &status == b"OKAY" {
        Ok(body)
    } else {
        anyhow::bail!(
            "adb host command {cmd:?} failed ({status:?}): {}",
            String::from_utf8_lossy(&body)
        )
    }
}

fn read_adb_body(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    if stream.read_exact(&mut len_buf).is_err() {
        return Ok(Vec::new());
    }
    let len = usize::from_str_radix(&String::from_utf8_lossy(&len_buf), 16).unwrap_or(0);
    let mut body = vec![0u8; len];
    if len > 0 {
        stream.read_exact(&mut body)?;
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn free_addr() -> SocketAddrV4 {
        let l = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)
    }

    #[test]
    fn tcp_open_detects_listener() {
        let addr = free_addr();
        assert!(!tcp_open(addr));
        let listener = std::net::TcpListener::bind(addr).unwrap();
        assert!(tcp_open(addr));
        drop(listener);
        assert!(!tcp_open(addr));
    }

    #[test]
    fn adb_host_command_fails_when_server_down() {
        let err = adb_host_command(free_addr(), "host:version").unwrap_err();
        assert!(err.to_string().contains("unreachable"));
    }
}
