use serde_json::Value;

use super::{VideoEntry, YoutubeResolver};

const SEARCH_ENDPOINT: &str =
    "https://www.youtube.com/youtubei/v1/search?key=AIzaSyAO_FJ2SlqU8Q4STEHLGCilw_Y9_11qcW8";
// ponytail: static client version, bump if requests start getting rejected
const CLIENT_VERSION: &str = "2.20241202.00.00";

impl YoutubeResolver {
    pub(crate) async fn search(&self, query: &str) -> Vec<VideoEntry> {
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
            log::info!(
                "youtube: searching innertube hl={} gl={} for {:?}",
                hl,
                gl,
                query
            );
        } else {
            log::debug!("youtube: searching innertube for {:?}", query);
        }

        let resp = self
            .client
            .post(SEARCH_ENDPOINT)
            .header(
                "User-Agent",
                concat!("wsa_rpc_bridge/", env!("CARGO_PKG_VERSION")),
            )
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
                log::warn!(
                    "youtube: failed to decode search response: {e} body={:?}",
                    snippet
                );
                return Vec::new();
            }
        };

        collect_videos(&json)
    }
}

pub(crate) fn collect_videos(value: &Value) -> Vec<VideoEntry> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

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
}
