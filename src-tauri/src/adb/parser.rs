use crate::models::MediaInfo;

const PLAYBACK_STATE_PLAYING: u8 = 3;

fn normalize_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn extract_package(line: &str) -> Option<String> {
    let b = line.as_bytes();
    if b.len() < 5 { return None; }
    if b[0] != b' ' || b[1] != b' ' || b[2] != b' ' || b[3] != b' ' { return None; }
    if b[4] == b' ' { return None; }

    let content = &line[4..];
    let mut parts = content.split_whitespace();
    let _first = parts.next()?;
    let second = parts.next()?;
    second.split('/').next().map(|s| s.to_string())
}

fn is_active_line(trimmed: &str) -> bool {
    normalize_whitespace(trimmed) == "active=true"
}

fn extract_state_data(trimmed: &str) -> Option<(u8, u64)> {
    let pos = trimmed.find("state=PlaybackState")?;
    let after = &trimmed[pos + "state=PlaybackState".len()..];
    let after_brace = after.trim_start().strip_prefix('{')?;

    let mut state_val = None;
    let mut pos_val = None;
    for part in after_brace.split(',') {
        let mut kv = part.trim().splitn(2, '=');
        let key = kv.next()?.trim();
        let val = kv.next()?.trim();
        match key {
            "state" => state_val = val.parse::<u8>().ok(),
            "position" => pos_val = val.parse::<u64>().ok(),
            _ => {}
        }
        if state_val.is_some() && pos_val.is_some() {
            break;
        }
    }
    Some((state_val?, pos_val?))
}

fn extract_description_raw(trimmed: &str) -> Option<String> {
    if !trimmed.trim_start().starts_with("metadata:") {
        return None;
    }
    let desc_start = trimmed.find("description=")?;
    Some(trimmed[desc_start + "description=".len()..].to_string())
}

pub fn parse_media_session(output: &str) -> Option<MediaInfo> {
    let lines: Vec<&str> = output.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        let package = match extract_package(line) {
            Some(p) => p,
            None => {
                i += 1;
                continue;
            }
        };

        log::debug!("ADB parser: found session package={}", package);

        let mut is_active = false;
        let mut title = String::new();
        let mut artist = String::new();
        let mut album = String::new();
        let mut position = None;
        let mut is_playing = false;

        i += 1;
        while i < lines.len() {
            let prop = lines[i];

            let trimmed = prop.trim();

            if trimmed.is_empty() {
                i += 1;
                continue;
            }

            if !prop.starts_with("      ") {
                break;
            }

            if is_active_line(trimmed) {
                log::debug!("ADB parser: session {} is active", package);
                is_active = true;
            }

            if let Some((state_val, pos)) = extract_state_data(trimmed) {
                is_playing = state_val == PLAYBACK_STATE_PLAYING;
                position = Some(pos);
                log::debug!(
                    "ADB parser: state={}, position={:?}",
                    state_val,
                    position,
                );
            }

            if let Some(raw_desc) = extract_description_raw(trimmed) {
                if let Some(parsed) = parse_description(&raw_desc) {
                    title = parsed.0;
                    artist = parsed.1;
                    album = parsed.2;
                    log::debug!("ADB parser: metadata title={:?}, artist={:?}", title, artist);
                }
            }

            i += 1;
        }

        if is_active || !title.is_empty() {
            let result = MediaInfo {
                title,
                artist,
                album,
                package_name: package,
                thumbnail_url: None,
                position,
                duration: None,
                display_name: None,
                is_playing,
            };
            if !result.title.is_empty() {
                log::info!(
                    "ADB parser: active={}, title={:?}, artist={:?}, playing={}",
                    is_active,
                    result.title,
                    result.artist,
                    result.is_playing,
                );
                return Some(result);
            }
        }
    }

    log::debug!("ADB parser: no active session found");
    None
}

fn parse_description(desc: &str) -> Option<(String, String, String)> {
    let parts: Vec<&str> = desc.rsplitn(3, ", ").collect();
    let n = parts.len();
    let title = parts.last()?.trim().to_string();
    let artist = (n >= 2)
        .then(|| parts[n - 2].trim())
        .filter(|s| *s != "null")
        .unwrap_or("")
        .to_string();
    let album = (n >= 3)
        .then(|| parts[n - 3].trim())
        .filter(|s| *s != "null")
        .unwrap_or("")
        .to_string();
    Some((title, artist, album))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_real_output() {
        let output = r#"MEDIA SESSION SERVICE (dumpsys media_session)

4 sessions listeners.
Global priority session is null
User Records:
Record for full_user=0
  Volume key long-press listener: null
  Volume key long-press listener package: null
  Media key listener: null
  Media key listener package: null
  OnMediaKeyEventDispatchedListener: added 0 listener(s)
  OnMediaKeyEventSessionChangedListener: added 0 listener(s)
  Last MediaButtonReceiver: null
  Media button session is com.example.app/androidx.media3.session.id. (userId=0)
  Sessions Stack - have 1 sessions:
    androidx.media3.session.id. com.example.app/androidx.media3.session.id. (userId=0)
      ownerPid=1700, ownerUid=10084, userId=0
      package=com.example.app
      launchIntent=PendingIntent{4dc6dba: PendingIntentRecord{d1a36eb com.example.app startActivity (allowlist: f26c4f4:+30s0ms/0/NOTIFICATION_SERVICE/NotificationManagerService)}}
      mediaButtonReceiver=null
      active=true
      flags=7
      rating type=0
      controllers: 1
      state=PlaybackState {state=3, position=33026, buffered position=33026, speed=1.0, updated=4824263, actions=3141555, custom actions=[], active item id=6, error=null}
      audioAttrs=AudioAttributes: usage=USAGE_MEDIA content=CONTENT_TYPE_MUSIC flags=0x800 tags= bundle=null
      volumeType=1, controlType=2, max=0, current=0
      metadata: size=15, description=楽曲タイトル / アーティストA, アーティストB, null
      queueTitle=null, size=338
Audio playback (lastly played comes first)
  uid=10084 packages=com.example.app 
Media session config:
  media_button_receiver_fgs_allowlist_duration_ms: [cur: 10000, def: 10000]
  media_session_calback_fgs_allowlist_duration_ms: [cur: 10000, def: 10000]
  media_session_callback_fgs_while_in_use_temp_allow_duration_ms: [cur: 10000, def: 10000]
"#;
        let result = parse_media_session(output);
        assert!(result.is_some(), "Should parse media info");
        let info = result.unwrap();
        assert_eq!(info.title, "楽曲タイトル / アーティストA");
        assert_eq!(info.artist, "アーティストB");
        assert_eq!(info.album, "");
        assert_eq!(info.package_name, "com.example.app");
        assert_eq!(info.thumbnail_url, None);
        assert!(info.is_playing);
        assert!(info.position.is_some());
        assert!(info.position.unwrap() > 0);
        assert!(info.duration.is_none());
    }

    #[test]
    fn test_no_session() {
        let output = "no sessions here";
        assert!(parse_media_session(output).is_none());
    }

    #[test]
    fn test_parse_description_normal() {
        let (title, artist, album) =
            parse_description("Song Title / Artist Name, Album Name, null").unwrap();
        assert_eq!(title, "Song Title / Artist Name");
        assert_eq!(artist, "Album Name");
        assert_eq!(album, "");
    }

    #[test]
    fn test_parse_description_japanese() {
        let (title, artist, album) = parse_description(
            "楽曲タイトル / アーティストA, アーティストB, null",
        )
        .unwrap();
        assert_eq!(title, "楽曲タイトル / アーティストA");
        assert_eq!(artist, "アーティストB");
        assert_eq!(album, "");
    }

    #[test]
    fn test_parse_description_without_slash() {
        let (title, artist, album) =
            parse_description("MV「タイトル」アーティストA feat.サンプル, アーティストA, null").unwrap();
        assert_eq!(title, "MV「タイトル」アーティストA feat.サンプル");
        assert_eq!(artist, "アーティストA");
        assert_eq!(album, "");
    }

    #[test]
    fn test_parse_description_only_title() {
        let (title, artist, album) = parse_description("Just a Title").unwrap();
        assert_eq!(title, "Just a Title");
        assert_eq!(artist, "");
        assert_eq!(album, "");
    }
}
