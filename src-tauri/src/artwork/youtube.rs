use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::artwork::ArtworkResolver;
use crate::models::MediaInfo;

const SEARCH_ENDPOINT: &str = "https://www.youtube.com/youtubei/v1/search";
// ponytail: static client version, bump if YouTube starts rejecting it
const CLIENT_VERSION: &str = "2.20250101.00.00";

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

        // ponytail: quoted first — YouTube treats "-x" as an exclusion operator; quoting disables it.
        // Plain title kept as fallback for phrase-mismatch cases.
        for query in [format!("\"{}\"", info.title), info.title.clone()] {
            let videos = self.search(&query).await;
            if let Some(id) = pick_video(&videos, &info.title, &info.artist) {
                let url = format!("https://i.ytimg.com/vi/{id}/mqdefault.jpg");
                log::info!("youtube: resolved thumbnail: {}", url);
                return Some(url);
            }
            log::debug!(
                "youtube: no match for query {:?} ({} results)",
                query,
                videos.len()
            );
        }
        log::debug!("youtube: no matching video for {:?}", info.title);
        None
    }
}

impl YoutubeResolver {
    async fn search(&self, query: &str) -> Vec<VideoEntry> {
        let body = serde_json::json!({
            "context": {
                "client": {
                    "clientName": "WEB",
                    "clientVersion": CLIENT_VERSION,
                }
            },
            "query": query,
        });

        log::debug!("youtube: searching innertube for {:?}", query);

        let resp = self
            .client
            .post(SEARCH_ENDPOINT)
            .header("User-Agent", concat!("wsa_rpc_bridge/", env!("CARGO_PKG_VERSION")))
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
            log::warn!("youtube: search API returned {}", status);
            return Vec::new();
        }

        let json: Value = match resp.json().await {
            Ok(j) => j,
            Err(e) => {
                log::warn!("youtube: failed to decode search response: {e}");
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

// ponytail: lowercase + strip bracketed decorations, no NFKC; add unicode-normalization if width variants bite
fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;
    let mut prev_space = true;
    for c in s.chars() {
        match c {
            '(' | '[' | '\u{3010}' | '\u{FF08}' => depth += 1,
            ')' | ']' | '\u{3011}' | '\u{FF09}' => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
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
        out.push(full);
    }
    for part in artist.split([',', '\u{3001}', '\u{FF0C}']) {
        let n = normalize(part);
        if !n.is_empty() && !out.contains(&n) {
            out.push(n);
        }
    }
    out
}

fn pick_video<'a>(videos: &'a [VideoEntry], title: &str, artist: &str) -> Option<&'a str> {
    // Tier 1: exact author + exact title
    if let Some(v) = videos.iter().find(|(_, t, a, _)| t == title && a == artist) {
        return Some(&v.0);
    }

    // Tier 2: unique exact title
    let exact: Vec<&VideoEntry> = videos.iter().filter(|(_, t, _, _)| t == title).collect();
    if exact.len() == 1 {
        return Some(&exact[0].0);
    }

    let nt = normalize(title);
    if nt.is_empty() {
        return None;
    }
    let candidates = artist_candidates(artist);

    let title_hit =
        |t: &str| !normalize(t).is_empty() && (normalize(t).contains(&nt) || nt.contains(&normalize(t)));
    let author_hit = move |a: &str| {
        if candidates.is_empty() {
            return true;
        }
        let m = normalize(a);
        if m.is_empty() {
            return false;
        }
        candidates.iter().any(|c| m.contains(c.as_str()) || c.contains(m.as_str()))
    };

    // Tier 3: fuzzy title + author match, most viewed wins (first in search order breaks ties)
    let mut best: Option<&VideoEntry> = None;
    for v in videos.iter() {
        if title_hit(&v.1) && author_hit(&v.2) && best.map_or(true, |b| v.3 > b.3) {
            best = Some(v);
        }
    }
    if let Some(v) = best {
        return Some(&v.0);
    }

    // Tier 4: unique fuzzy title regardless of author
    let hits: Vec<&VideoEntry> = videos.iter().filter(|v| title_hit(&v.1)).collect();
    if hits.len() == 1 {
        return Some(&hits[0].0);
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
                                                "videoId": "dQw4w9WgXcQ",
                                                "title": { "runs": [{ "text": "Rick Astley - Never Gonna Give You Up (Official Video)" }] },
                                                "ownerText": { "runs": [{ "text": "Rick Astley" }] },
                                                "viewCountText": { "simpleText": "1.6B views" }
                                            }},
                                            { "videoRenderer": {
                                                "videoId": "abc123XYZ_-",
                                                "title": { "simpleText": "Never Gonna Give You Up (Official Audio)" },
                                                "longBylineText": { "runs": [{ "text": "Rick Astley - Topic" }] },
                                                "shortViewCountText": { "simpleText": "320万回視聴" }
                                            }},
                                            { "videoRenderer": {
                                                "videoId": "xyz789_____",
                                                "title": { "runs": [{ "text": "Never Gonna Give You Up (Live)" }] },
                                                "ownerText": { "runs": [{ "text": "Rick Astley" }] },
                                                "viewCountText": { "simpleText": "12M views" }
                                            }},
                                            { "videoRenderer": {
                                                "videoId": "dupe0000000",
                                                "title": { "runs": [{ "text": "Never Gonna Give You Up (Live)" }] },
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
                "dQw4w9WgXcQ".into(),
                "Rick Astley - Never Gonna Give You Up (Official Video)".into(),
                "Rick Astley".into(),
                1_600_000_000
            )
        );
        assert_eq!(videos[1].1, "Never Gonna Give You Up (Official Audio)");
        assert_eq!(videos[1].2, "Rick Astley - Topic");
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
        assert_eq!(normalize("Never Gonna Give You Up (Official Video)"), "never gonna give you up");
        assert_eq!(normalize("タイトル【特典付き】"), "タイトル");
        assert_eq!(normalize("  A   B  "), "a b");
        assert_eq!(normalize("（全角）"), "");
    }

    #[test]
    fn pick_tier1_exact_author_beats_more_viewed_fuzzy() {
        let videos = collect_videos(&fixture());
        assert_eq!(
            pick_video(&videos, "Never Gonna Give You Up (Live)", "Other Channel"),
            Some("dupe0000000")
        );
    }

    #[test]
    fn pick_tier2_unique_exact_title_even_with_author_mismatch() {
        let videos = collect_videos(&fixture());
        assert_eq!(
            pick_video(&videos, "Rick Astley - Never Gonna Give You Up (Official Video)", "リック・アストリー"),
            Some("dQw4w9WgXcQ")
        );
    }

    #[test]
    fn pick_tier3_fuzzy_author_match_picks_most_viewed() {
        let videos = collect_videos(&fixture());
        assert_eq!(
            pick_video(&videos, "Never Gonna Give You Up", "Rick Astley"),
            Some("dQw4w9WgXcQ")
        );
    }

    #[test]
    fn pick_rejects_when_no_author_match_and_fuzzy_hits_are_many() {
        let videos = collect_videos(&fixture());
        assert_eq!(pick_video(&videos, "Never Gonna Give You Up", "無関係なチャンネル"), None);
        assert_eq!(pick_video(&videos, "存在しないタイトル", "Rick Astley"), None);
    }

    fn entry(id: &str, title: &str, author: &str, views: u64) -> VideoEntry {
        (id.into(), title.into(), author.into(), views)
    }

    #[test]
    fn pick_collab_jp_artist_matches_each_channel() {
        // 名前順序に依らずauthor候補がヒットし、再生回数最大の方が選ばれる
        let a_first = vec![
            entry("v1", "コラボ楽曲 (Music Video)", "星野源", 500_000_000),
            entry("v2", "コラボ楽曲 (Official Audio)", "米津玄師 - Topic", 100_000_000),
        ];
        assert_eq!(pick_video(&a_first, "コラボ楽曲", "星野源、米津玄師"), Some("v1"));

        let b_first = vec![
            entry("v1", "コラボ楽曲 (Music Video)", "星野源", 100_000_000),
            entry("v2", "コラボ楽曲 (Official Audio)", "米津玄師 - Topic", 500_000_000),
        ];
        assert_eq!(pick_video(&b_first, "コラボ楽曲", "星野源、米津玄師"), Some("v2"));
    }

    #[test]
    fn pick_collab_en_artist_matches_topic_channel() {
        let videos = vec![
            entry("v1", "Collab Song (Music Video)", "Artist A", 100_000_000),
            entry("v2", "Collab Song (Official Audio)", "Artist B - Topic", 50_000_000),
        ];
        assert_eq!(pick_video(&videos, "Collab Song", "Artist A, Artist B"), Some("v1"));
    }

    #[test]
    fn pick_full_artist_string_survives_commas_in_name() {
        let videos = vec![entry("ewf", "September (Remastered)", "Earth, Wind & Fire", 900_000)];
        assert_eq!(pick_video(&videos, "September", "Earth, Wind & Fire"), Some("ewf"));
    }
}
