use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::artwork::{ArtworkResolver, PLACEHOLDER_URL};
use crate::models::MediaInfo;

const SEARCH_ENDPOINT: &str =
    "https://www.youtube.com/youtubei/v1/search?key=AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";
// ponytail: static client version, bump if requests start getting rejected
const CLIENT_VERSION: &str = "2.20241202.00.00";

type VideoEntry = (String, String, String, u64); // (videoId, title, author, views)

pub struct YoutubeResolver {
    client: Client,
    package: &'static str,
}

impl YoutubeResolver {
    pub fn new(package: &'static str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self { client, package }
    }
}

#[async_trait]
impl ArtworkResolver for YoutubeResolver {
    fn package_name(&self) -> &str {
        self.package
    }

    async fn resolve(&self, info: &MediaInfo) -> Option<String> {
        if info.title.trim().is_empty() {
            return None;
        }
        // ponytail: artist empty skip — some apps send title first, artist 100ms later; empty artist causes generic hits
        if info.artist.trim().is_empty() {
            return None;
        }

        // ponytail: quoted first — search treats "-x" as an exclusion operator; quoting disables it.
        // Plain title kept as fallback for phrase-mismatch cases.
        // ponytail: 2↔3 swap — artist付きplainを先にし汎用タイトルの誤検出を防ぐ
        // ponytail: search words are washed first (video-type tail, reissue tail,
        // quality tail, channel tail, second and later names); raw values kept for logs only
        let washed_title = wash_title(&info.title);
        let washed_artist = wash_artist(&info.artist);
        let title_q = if washed_title.is_empty() {
            info.title.trim().to_string()
        } else {
            washed_title.clone()
        };
        let artist_q = if washed_artist.is_empty() {
            info.artist.trim().to_string()
        } else {
            washed_artist.clone()
        };
        // ponytail: name-title order first — single keyword hits the official upload better
        let queries: Vec<String> = if artist_q.trim().is_empty() {
            vec![format!("\"{title_q}\""), title_q.clone()]
        } else {
            vec![
                format!("\"{artist_q} - {title_q}\""),
                format!("\"{title_q}\""),
                format!("{title_q} {artist_q}"),
                title_q.clone(),
            ]
        };
        // ponytail: 作者不一致は確定的なのでplaceholder明示返却（キャッシュ可）。空結果は一時的失敗としてNone維持。
        let mut had_results = false;
        for query in queries.iter() {
            let videos = self.search(query).await;
            if !videos.is_empty() {
                had_results = true;
            }
            if let Some((id, tier)) = pick_video(&videos, &title_q, &artist_q) {
                let url = format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg");
                // TMPLOG: 誤検出切り分け用一時ログ（原因特定後に削除）
                if cfg!(debug_assertions) {
                    if let Some(m) = videos.iter().find(|v| v.0 == id) {
                        log::info!(
                            "youtube: TMPLOG matched title={:?} author={:?} views={} is_topic={}",
                            m.1,
                            m.2,
                            m.3,
                            m.2.to_lowercase().contains("topic")
                        );
                    }
                    for (i, v) in videos.iter().take(3).enumerate() {
                        log::info!(
                            "youtube: TMPLOG candidate {} id={} title={:?} author={:?} views={}",
                            i, v.0, v.1, v.2, v.3
                        );
                    }
                }
                if cfg!(debug_assertions) {
                    log::info!(
                        "youtube: resolved videoId={} tier={} query={:?} title={:?} artist={:?}",
                        id, tier, query, info.title, info.artist
                    );
                } else {
                    log::info!("youtube: resolved thumbnail: {}", url);
                }
                return Some(url);
            }
            if cfg!(debug_assertions) {
                log::info!(
                    "youtube: no match for query {:?} ({} results)",
                    query,
                    videos.len()
                );
                if videos.is_empty() {
                    log::info!("youtube: no results - likely API key/version rejected for query {:?}", query);
                } else {
                    for (i, v) in videos.iter().take(3).enumerate() {
                        log::info!(
                            "youtube: candidate {} id={} title={:?} author={:?} views={}",
                            i, v.0, v.1, v.2, v.3
                        );
                    }
                }
            } else {
                log::debug!(
                    "youtube: no match for query {:?} ({} results)",
                    query,
                    videos.len()
                );
            }
        }
        if cfg!(debug_assertions) {
            log::info!("youtube: no matching video for {:?}", info.title);
        } else {
            log::debug!("youtube: no matching video for {:?}", info.title);
        }
        if had_results {
            return Some(PLACEHOLDER_URL.to_string());
        }
        None
    }
}

impl YoutubeResolver {
    async fn search(&self, query: &str) -> Vec<VideoEntry> {
        // ponytail: use system locale for hl/gl so author names match user's language
        let locale = sys_locale::get_locale().unwrap_or_else(|| "ja-JP".to_string());
        let (hl, gl) = {
            let mut parts = locale.split(|c| c == '-' || c == '_');
            let hl = parts.next().unwrap_or("ja").to_ascii_lowercase();
            let hl = if hl.is_empty() { "ja".to_string() } else { hl };
            let gl_raw = parts.next().unwrap_or(if hl == "ja" { "JP" } else { "US" });
            (hl, gl_raw.to_ascii_uppercase())
        };
        let accept_lang = format!("{}, en;q=0.5", locale);
        let body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB",
                    "clientVersion": CLIENT_VERSION,
                    "hl": hl,
                    "gl": gl,
                }
            },
            "query": query,
        });

        if cfg!(debug_assertions) {
            log::info!("youtube: searching innertube hl={} gl={} for {:?}", hl, gl, query);
        } else {
            log::debug!("youtube: searching innertube for {:?}", query);
        }

        let resp = self
            .client
            .post(SEARCH_ENDPOINT)
            .header("User-Agent", concat!("wsa_rpc_bridge/", env!("CARGO_PKG_VERSION")))
            .header("Content-Type", "application/json")
            .header("Origin", "https://www.youtube.com")
            .header("Referer", "https://www.youtube.com/")
            .header("Accept-Language", accept_lang)
            .json(&body)
            .send()
            .await;

        let resp = match resp {
            Ok(r) => r,
            Err(e) => {
                log::warn!("youtube: search request failed: {e}");
                return Vec::new();
            }
        };

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            let snippet = body.chars().take(400).collect::<String>();
            log::warn!("youtube: search API returned {} body={:?}", status, snippet);
            return Vec::new();
        }

        let text = match resp.text().await {
            Ok(t) => t,
            Err(e) => {
                log::warn!("youtube: failed to read search response: {e}");
                return Vec::new();
            }
        };
        let json: Value = match serde_json::from_str(&text) {
            Ok(j) => j,
            Err(e) => {
                let snippet = text.chars().take(400).collect::<String>();
                log::warn!("youtube: failed to decode search response: {e} body={:?}", snippet);
                return Vec::new();
            }
        };

        collect_videos(&json)
    }
}

fn collect_videos(value: &Value) -> Vec<VideoEntry> {
    match value {
        Value::Object(map) => match map.get("videoRenderer") {
            Some(r) => parse_video_renderer(r).into_iter().collect(),
            None => map.values().flat_map(collect_videos).collect(),
        },
        Value::Array(items) => items.iter().flat_map(collect_videos).collect(),
        _ => Vec::new(),
    }
}

fn parse_video_renderer(r: &Value) -> Option<VideoEntry> {
    let id = r["videoId"].as_str()?;
    let title = text(&r["title"])?;
    let author = r["ownerText"]["runs"][0]["text"]
        .as_str()
        .or_else(|| r["longBylineText"]["runs"][0]["text"].as_str())?;
    Some((
        id.to_string(),
        title.to_string(),
        author.to_string(),
        views_of(r),
    ))
}

fn text(v: &Value) -> Option<&str> {
    v["simpleText"]
        .as_str()
        .or_else(|| v["runs"][0]["text"].as_str())
}

fn views_of(r: &Value) -> u64 {
    for key in ["viewCountText", "shortViewCountText"] {
        if let Some(t) = text(&r[key]) {
            let v = parse_views(t);
            if v > 0 {
                return v;
            }
        }
    }
    0
}

// ponytail: localized count parser (万/億/K/M/B), extend if other locales appear
fn parse_views(text: &str) -> u64 {
    let mut num = String::new();
    let mut unit = String::new();
    let mut past_digits = false;
    for c in text.chars() {
        if c.is_ascii_digit() || c == '.' {
            num.push(c);
            past_digits = true;
        } else if past_digits && !c.is_whitespace() && c != ',' {
            unit.push(c);
        }
    }
    let n: f64 = num.parse().unwrap_or(0.0);
    let unit = unit.to_lowercase();
    let mult = if unit.starts_with('万') {
        1e4
    } else if unit.starts_with('億') || unit.starts_with('亿') {
        1e8
    } else if unit.starts_with('k') {
        1e3
    } else if unit.starts_with('m') {
        1e6
    } else if unit.starts_with('b') {
        1e9
    } else {
        1.0
    };
    (n * mult) as u64
}

// ponytail: wash before search — drop trailing tails that pollute matching
fn wash_title(raw: &str) -> String {
    let mut cur = raw.trim().to_string();
    loop {
        let next = strip_one_trailing_group(&cur);
        if next.len() == cur.len() {
            break;
        }
        cur = next;
        if cur.trim().is_empty() {
            return String::new();
        }
    }
    cur.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_one_trailing_group(s: &str) -> String {
    const PAIRS: [(char, char); 6] = [
        ('(', ')'),
        ('[', ']'),
        ('\u{3010}', '\u{3011}'),
        ('\u{300c}', '\u{300d}'),
        ('\u{300e}', '\u{300f}'),
        ('\u{ff08}', '\u{ff09}'),
    ];
    let trimmed = s.trim_end();
    let close = match trimmed.chars().last() {
        Some(c) => c,
        None => return s.to_string(),
    };
    let open = match PAIRS.iter().find(|(_, cl)| *cl == close) {
        Some((op, _)) => *op,
        None => return s.to_string(),
    };
    let open_at = match trimmed.rfind(open) {
        Some(i) => i,
        None => return s.to_string(),
    };
    let inner = &trimmed[open_at + open.len_utf8()..trimmed.len() - close.len_utf8()];
    if !is_washable_tail(inner) {
        return s.to_string();
    }
    trimmed[..open_at].trim_end().to_string()
}

fn is_washable_tail(inner: &str) -> bool {
    const LONG: [&str; 8] = [
        "official", "video", "audio", "visual", "lyric", "remaster", "mono", "stereo",
    ];
    const SHORT: [&str; 5] = ["mv", "hq", "hd", "4k", "8k"];
    let low = inner.to_lowercase();
    if LONG.iter().any(|m| low.contains(m)) {
        return true;
    }
    low.split(|c: char| !c.is_alphanumeric()).any(|tok| SHORT.contains(&tok))
}

// ponytail: channel tail + second and later names cut for search words only
fn wash_artist(raw: &str) -> String {
    let mut cur = raw.trim().to_string();
    if cur.to_lowercase().ends_with("- topic") {
        let cut = cur.len() - "- topic".len();
        cur = cur[..cut].trim_end().trim_end_matches('-').trim_end().to_string();
    }
    let low = cur.to_lowercase();
    let mut cut_at: Option<usize> = None;
    const HARD: [&str; 6] = [" & ", ",", "\u{3001}", "\u{ff0c}", ";", "\u{ff1b}"];
    for sep in HARD {
        if let Some(i) = cur.find(sep) {
            if i > 0 && cut_at.map_or(true, |c| i < c) {
                cut_at = Some(i);
            }
        }
    }
    const SOFT: [&str; 7] = [" feat", " ft", " with ", " vs ", " plus ", " x ", " \u{00d7} "];
    for sep in SOFT {
        if let Some(i) = low.find(sep) {
            if i > 0 && cut_at.map_or(true, |c| i < c) {
                cut_at = Some(i);
            }
        }
    }
    if let Some(i) = cut_at {
        cur = cur[..i].to_string();
    }
    cur.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ponytail: lowercase + normalize brackets/separators to space, keep inner text (version distinction like feat)
fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = true;
    for c in s.chars() {
        match c {
            '(' | '[' | '\u{3010}' | '\u{FF08}' | '\u{300E}' | '\u{300C}' | ')' | ']' | '\u{3011}' | '\u{FF09}' | '\u{300F}' | '\u{300D}' | '"' | '\'' | '`' | '“' | '”' | '‘' | '’' => {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
                continue;
            }
            // separators like dash/colon that often split title/artist: normalize to space
            '-' | '–' | '—' | ':' | '：' | '・' | '.' | '·' | '_' | '/' | '／' => {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            }
            c if c.is_whitespace() => {
                if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            }
            c => {
                out.extend(c.to_lowercase());
                prev_space = false;
            }
        }
    }
    out.trim_end().to_string()
}

// ponytail: hypothesis list — full string first, then comma-split parts; no need to decide which split is right
fn artist_candidates(artist: &str) -> Vec<String> {
    let mut out = Vec::new();
    let full = normalize(artist);
    if !full.is_empty() {
        out.push(full.clone());
        let nospace = full.replace(' ', "").replace('　', "");
        if !nospace.is_empty() && !out.contains(&nospace) {
            out.push(nospace);
        }
    }
    for part in artist.split([',', '\u{3001}', '\u{FF0C}']) {
        let n = normalize(part);
        if !n.is_empty() && !out.contains(&n) {
            out.push(n.clone());
            let ns = n.replace(' ', "").replace('　', "");
            if !ns.is_empty() && !out.contains(&ns) {
                out.push(ns);
            }
        }
    }
    out
}

fn pick_video<'a>(videos: &'a [VideoEntry], title: &str, artist: &str) -> Option<(&'a str, u8)> {
    // Tier 1: exact author + exact title
    if let Some(v) = videos.iter().find(|(_, t, a, _)| t == title && a == artist) {
        return Some((&v.0, 1));
    }

    // Tier 1.5: author contains (loose) + title hit, most viewed — official prior
    let full_norm = normalize(artist);
    if !full_norm.is_empty() {
        let nt15 = normalize(title);
        if !nt15.is_empty() {
            let title_hit15 = |t: &str| {
                let mt = normalize(t);
                !mt.is_empty() && (mt.contains(&nt15) || nt15.contains(&mt))
            };
            let full_ns = full_norm.replace(' ', "").replace('　', "");
            let author_loose = |a: &str| {
                let m = normalize(a);
                if m.is_empty() {
                    return false;
                }
                let mn = m.replace(' ', "").replace('　', "");
                m.contains(&full_norm)
                    || full_norm.contains(&m)
                    || mn.contains(&full_ns)
                    || full_ns.contains(&mn)
                    || m.contains(&full_ns)
                    || full_ns.contains(&mn)
            };
            let mut best15: Option<&VideoEntry> = None;
            for v in videos.iter() {
                if title_hit15(&v.1) && author_loose(&v.2) && best15.map_or(true, |b| v.3 > b.3) {
                    best15 = Some(v);
                }
            }
            if let Some(v) = best15 {
                return Some((&v.0, 15));
            }
        }
    }

    // Tier 2: fuzzy title + author match, most viewed wins
    let nt = normalize(title);
    if nt.is_empty() {
        return None;
    }
    let candidates = artist_candidates(artist);

    let title_hit =
        |t: &str| !normalize(t).is_empty() && (normalize(t).contains(&nt) || nt.contains(&normalize(t)));
    let candidates_for_author = candidates.clone();
    let author_hit = move |a: &str| {
        if candidates_for_author.is_empty() {
            return true;
        }
        let m = normalize(a);
        if m.is_empty() {
            return false;
        }
        let mn = m.replace(' ', "").replace('　', "");
        candidates_for_author.iter().any(|c| {
            let cn = c.replace(' ', "").replace('　', "");
            m.contains(c.as_str()) || c.contains(m.as_str()) || mn.contains(&cn) || cn.contains(&mn)
        })
    };

    // ponytail: removed old Tier 2 (unique exact title w/o author check) — auto-generated
    // uploads with clean titles were beating official uploads.
    // fallback: if author field is localized, check title contains artist
    // also check raw title for bracketed artist names which normalize strips
    // ponytail: title言及はtopic含有限定 — カバーの誤検出防止
    let is_topic = |a: &str| a.to_lowercase().contains("topic");
    let title_contains_artist = |t: &str| {
        if candidates.is_empty() {
            return false;
        }
        // raw check for bracketed artist
        let t_lower = t.to_lowercase();
        let artist_lower = artist.to_lowercase();
        if !artist_lower.trim().is_empty() && t_lower.contains(&artist_lower) {
            return true;
        }
        for cand in &candidates {
            if !cand.is_empty() && t_lower.contains(&cand.to_lowercase()) {
                return true;
            }
        }
        let mt = normalize(t);
        if mt.is_empty() {
            return false;
        }
        let mt_nospace = mt.replace(' ', "").replace('　', "");
        candidates.iter().any(|c| {
            let cn = c.replace(' ', "").replace('　', "");
            mt.contains(c.as_str()) || c.contains(mt.as_str()) || mt_nospace.contains(&cn) || cn.contains(&mt_nospace)
        })
    };
    let mut best: Option<&VideoEntry> = None;
    for v in videos.iter() {
        if title_hit(&v.1)
            && (author_hit(&v.2) || (is_topic(&v.2) && title_contains_artist(&v.1)))
            && best.map_or(true, |b| v.3 > b.3)
        {
            best = Some(v);
        }
    }
    if let Some(v) = best {
        return Some((&v.0, 2));
    }

    // ponytail: Tier 3廃止 — 作者一致なしはplaceholderへリダイレクト（誤サムネ防止）
    // Tier 4: translation fallback — search TOP is same artist even if title mismatch
    if let Some(top) = videos.first() {
        if author_hit(&top.2) || (is_topic(&top.2) && title_contains_artist(&top.1)) {
            return Some((&top.0, 4));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture() -> Value {
        json!({
            "contents": {
                "twoColumnSearchResultsRenderer": {
                    "primaryContents": {
                        "sectionListRenderer": {
                            "contents": [
                                {
                                    "itemSectionRenderer": {
                                        "contents": [
                                            { "videoRenderer": {
                                                "videoId": "test0000001",
                                                "title": { "runs": [{ "text": "Artist A - Test Song (Official Video)" }] },
                                                "ownerText": { "runs": [{ "text": "Artist A" }] },
                                                "viewCountText": { "simpleText": "1.6B views" }
                                            }},
                                            { "videoRenderer": {
                                                "videoId": "abc123XYZ_-",
                                                "title": { "simpleText": "Test Song (Official Audio)" },
                                                "longBylineText": { "runs": [{ "text": "Artist A - Topic" }] },
                                                "shortViewCountText": { "simpleText": "320万回視聴" }
                                            }},
                                            { "videoRenderer": {
                                                "videoId": "xyz789_____",
                                                "title": { "runs": [{ "text": "Test Song (Live)" }] },
                                                "ownerText": { "runs": [{ "text": "Artist A" }] },
                                                "viewCountText": { "simpleText": "12M views" }
                                            }},
                                            { "videoRenderer": {
                                                "videoId": "dupe0000000",
                                                "title": { "runs": [{ "text": "Test Song (Live)" }] },
                                                "ownerText": { "runs": [{ "text": "Other Channel" }] },
                                                "viewCountText": { "simpleText": "1,234,567 views" }
                                            }}
                                        ]
                                    }
                                }
                            ]
                        }
                    }
                }
            }
        })
    }

    #[test]
    fn collect_walks_nested_renderers() {
        let videos = collect_videos(&fixture());
        assert_eq!(videos.len(), 4);
        assert_eq!(
            videos[0],
            (
                "test0000001".into(),
                "Artist A - Test Song (Official Video)".into(),
                "Artist A".into(),
                1_600_000_000
            )
        );
        assert_eq!(videos[1].1, "Test Song (Official Audio)");
        assert_eq!(videos[1].2, "Artist A - Topic");
        assert_eq!(videos[1].3, 3_200_000);
    }

    #[test]
    fn parse_views_handles_locales_and_units() {
        assert_eq!(parse_views("1,234,567 views"), 1_234_567);
        assert_eq!(parse_views("123万回視聴"), 1_230_000);
        assert_eq!(parse_views("1.5億回視聴"), 150_000_000);
        assert_eq!(parse_views("1.2M views"), 1_200_000);
        assert_eq!(parse_views("987K views"), 987_000);
        assert_eq!(parse_views("2B views"), 2_000_000_000);
        assert_eq!(parse_views("視聴回数なし"), 0);
    }

    #[test]
    fn normalizes_case_brackets_and_spaces() {
        assert_eq!(
            normalize("Test Song (Official Video)"),
            "test song official video"
        );
        assert_eq!(normalize("タイトル【特典付き】"), "タイトル 特典付き");
        assert_eq!(normalize("  A   B  "), "a b");
        assert_eq!(normalize("（全角）"), "全角");
    }

    #[test]
    fn pick_tier1_exact_author_beats_more_viewed_fuzzy() {
        let videos = collect_videos(&fixture());
        assert_eq!(
            pick_video(&videos, "Test Song (Live)", "Other Channel"),
            Some(("dupe0000000", 1))
        );
    }

    #[test]
    fn pick_tier2_fuzzy_author_match_picks_most_viewed() {
        let videos = collect_videos(&fixture());
        assert_eq!(
            pick_video(&videos, "Test Song", "Artist A"),
            Some(("test0000001", 2))
        );
    }

    #[test]
    fn pick_rejects_when_no_author_match_and_fuzzy_hits_are_many() {
        let videos = collect_videos(&fixture());
        assert_eq!(pick_video(&videos, "Test Song", "無関係なチャンネル"), None);
        assert_eq!(pick_video(&videos, "存在しないタイトル", "Artist A"), None);
    }

    #[test]
    fn pick_rejects_single_hit_without_author_match() {
        let videos = vec![entry("v1", "固有タイトル", "別人チャンネル", 100)];
        assert_eq!(pick_video(&videos, "固有タイトル", "本物作者"), None);
    }

    #[test]
    fn pick_rejects_cover_whose_title_mentions_artist() {
        let videos = vec![entry(
            "cover1",
            "テスト楽曲 (テスト作者) をフルートで吹いた場合 | Flute cover",
            "Cover Channel",
            999_999_999,
        )];
        assert_eq!(pick_video(&videos, "テスト楽曲", "テスト作者"), None);
    }

    #[test]
    fn pick_keeps_topic_rescue_via_title_mention() {
        let videos = vec![entry("t1", "テスト楽曲 - テスト作者", "Test Author - Topic", 10)];
        assert!(pick_video(&videos, "テスト楽曲", "テスト作者").is_some());
    }

    fn entry(id: &str, title: &str, author: &str, views: u64) -> VideoEntry {
        (id.into(), title.into(), author.into(), views)
    }

    #[test]
    fn pick_collab_jp_artist_matches_each_channel() {
        let a_first = vec![
            entry("v1", "テスト楽曲 (Music Video)", "テスト作者A", 500_000_000),
            entry("v2", "テスト楽曲 (Official Audio)", "テスト作者B - Topic", 100_000_000),
        ];
        assert_eq!(pick_video(&a_first, "テスト楽曲", "テスト作者A、テスト作者B"), Some(("v1", 2)));

        let b_first = vec![
            entry("v1", "テスト楽曲 (Music Video)", "テスト作者A", 100_000_000),
            entry("v2", "テスト楽曲 (Official Audio)", "テスト作者B - Topic", 500_000_000),
        ];
        assert_eq!(pick_video(&b_first, "テスト楽曲", "テスト作者A、テスト作者B"), Some(("v2", 2)));
    }

    #[test]
    fn pick_collab_en_artist_matches_topic_channel() {
        let videos = vec![
            entry("v1", "Collab Song (Music Video)", "Artist A", 100_000_000),
            entry("v2", "Collab Song (Official Audio)", "Artist B - Topic", 50_000_000),
        ];
        assert_eq!(pick_video(&videos, "Collab Song", "Artist A, Artist B"), Some(("v1", 2)));
    }

    #[test]
    fn pick_full_artist_string_survives_commas_in_name() {
        let videos = vec![entry("ewf", "Test Song (Remastered)", "Test Artist, Test Group", 900_000)];
        assert_eq!(pick_video(&videos, "Test Song", "Test Artist, Test Group"), Some(("ewf", 2)));
    }

    #[test]
    fn wash_title_strips_tails_only() {
        assert_eq!(wash_title("Song Name (Official Video)"), "Song Name");
        assert_eq!(wash_title("Song Name [Remastered]"), "Song Name");
        assert_eq!(wash_title("Song Name (HD)"), "Song Name");
        assert_eq!(wash_title("Song Name (Live)"), "Song Name (Live)");
        assert_eq!(wash_title("Song Name"), "Song Name");
    }

    #[test]
    fn wash_artist_keeps_first_only() {
        assert_eq!(wash_artist("Someone - Topic"), "Someone");
        assert_eq!(wash_artist("A, B"), "A");
        assert_eq!(wash_artist("A & B"), "A");
        assert_eq!(wash_artist("Solo"), "Solo");
    }
}
