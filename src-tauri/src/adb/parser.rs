use crate::models::MediaInfo;
use regex::Regex;

pub fn parse_media_session(output: &str) -> Option<MediaInfo> {
    let session_re = Regex::new(r"^\s{4}\S+\s+(\S+?)/").ok()?;
    let active_re = Regex::new(r"^\s*active\s*=\s*true").ok()?;
    let state_re = Regex::new(r"state=PlaybackState\s*\{state=(\d),\s*position=(\d+)").ok()?;
    let metadata_re = Regex::new(r"metadata:\s*size=\d+,\s*description=(.+)").ok()?;

    let lines: Vec<&str> = output.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        let package = match session_re.captures(line) {
            Some(c) => c[1].to_string(),
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

            if active_re.is_match(trimmed) {
                log::debug!("ADB parser: session {} is active", package);
                is_active = true;
            }

            if let Some(c) = state_re.captures(trimmed) {
                let state_val: u8 = c[1].parse().unwrap_or(0);
                is_playing = state_val == 3;
                position = c[2].parse::<u64>().ok();
                log::debug!(
                    "ADB parser: state={}, position={:?}",
                    state_val,
                    position,
                );
            }

            if let Some(c) = metadata_re.captures(trimmed) {
                if let Some(parsed) = parse_description(&c[1]) {
                    title = parsed.0;
                    artist = parsed.1;
                    album = parsed.2;
                    log::debug!("ADB parser: metadata title={:?}, artist={:?}", title, artist);
                }
            }

            i += 1;
        }

        if is_active || (!title.is_empty() && !is_active) {
            let result = MediaInfo {
                title,
                artist,
                album,
                position,
                duration: None,
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
  Volume key long-press listener package: 
  Media key listener: null
  Media key listener package: 
  OnMediaKeyEventDispatchedListener: added 0 listener(s)
  OnMediaKeyEventSessionChangedListener: added 0 listener(s)
  Last MediaButtonReceiver: null
  Media button session is jp.nicovideo.nicobox/androidx.media3.session.id. (userId=0)
  Sessions Stack - have 1 sessions:
    androidx.media3.session.id. jp.nicovideo.nicobox/androidx.media3.session.id. (userId=0)
      ownerPid=1700, ownerUid=10084, userId=0
      package=jp.nicovideo.nicobox
      launchIntent=PendingIntent{4dc6dba: PendingIntentRecord{d1a36eb jp.nicovideo.nicobox startActivity (allowlist: f26c4f4:+30s0ms/0/NOTIFICATION_SERVICE/NotificationManagerService)}}
      mediaButtonReceiver=null
      active=true
      flags=7
      rating type=0
      controllers: 1
      state=PlaybackState {state=3, position=33026, buffered position=33026, speed=1.0, updated=4824263, actions=3141555, custom actions=[], active item id=6, error=null}
      audioAttrs=AudioAttributes: usage=USAGE_MEDIA content=CONTENT_TYPE_MUSIC flags=0x800 tags= bundle=null
      volumeType=1, controlType=2, max=0, current=0
      metadata: size=15, description=BRIGHTNESS / Mwk feat.初音ミク, Mwk, null
      queueTitle=null, size=338
Audio playback (lastly played comes first)
  uid=10084 packages=jp.nicovideo.nicobox 
Media session config:
  media_button_receiver_fgs_allowlist_duration_ms: [cur: 10000, def: 10000]
  media_session_calback_fgs_allowlist_duration_ms: [cur: 10000, def: 10000]
  media_session_callback_fgs_while_in_use_temp_allow_duration_ms: [cur: 10000, def: 10000]
"#;
        let result = parse_media_session(output);
        assert!(result.is_some(), "Should parse media info");
        let info = result.unwrap();
        assert_eq!(info.title, "BRIGHTNESS / Mwk feat.初音ミク");
        assert_eq!(info.artist, "Mwk");
        assert_eq!(info.album, "");
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
            "ぜったいだった！！！！ / IA × 初音ミク, 内緒の秘密, null",
        )
        .unwrap();
        assert_eq!(title, "ぜったいだった！！！！ / IA × 初音ミク");
        assert_eq!(artist, "内緒の秘密");
        assert_eq!(album, "");
    }

    #[test]
    fn test_parse_description_without_slash() {
        let (title, artist, album) =
            parse_description("MV「乙女辞表」是 feat.初音ミク, 是, null").unwrap();
        assert_eq!(title, "MV「乙女辞表」是 feat.初音ミク");
        assert_eq!(artist, "是");
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

fn parse_description(desc: &str) -> Option<(String, String, String)> {
    let parts: Vec<&str> = desc.splitn(4, ", ").collect();
    let first = parts.first()?;
    let title = first.trim().to_string();
    let artist = parts
        .get(1)
        .map(|s| s.trim())
        .filter(|s| *s != "null")
        .unwrap_or("")
        .to_string();
    let album = parts
        .get(2)
        .map(|s| s.trim())
        .filter(|s| *s != "null")
        .unwrap_or("")
        .to_string();
    Some((title, artist, album))
}
