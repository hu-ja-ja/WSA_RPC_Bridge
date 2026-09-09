mod client;
mod matching;

use std::time::Duration;

use async_trait::async_trait;
use reqwest::Client;

use crate::artwork::{ArtworkResolver, PLACEHOLDER_URL};
use crate::models::MediaInfo;

use matching::{artist_candidates, normalize, wash_artist, wash_title};

pub(crate) type VideoEntry = (String, String, String, u64); // (videoId, title, author, views)

pub struct YoutubeResolver {
    pub(crate) client: Client,
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
                            i,
                            v.0,
                            v.1,
                            v.2,
                            v.3
                        );
                    }
                }
                if cfg!(debug_assertions) {
                    log::info!(
                        "youtube: resolved videoId={} tier={} query={:?} title={:?} artist={:?}",
                        id,
                        tier,
                        query,
                        info.title,
                        info.artist
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
                    log::info!(
                        "youtube: no results - likely API key/version rejected for query {:?}",
                        query
                    );
                } else {
                    for (i, v) in videos.iter().take(3).enumerate() {
                        log::info!(
                            "youtube: candidate {} id={} title={:?} author={:?} views={}",
                            i,
                            v.0,
                            v.1,
                            v.2,
                            v.3
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

fn pick_video<'a>(videos: &'a [VideoEntry], title: &str, artist: &str) -> Option<(&'a str, u8)> {
    // Tier 1: exact author + exact title
    if let Some(v) = videos.iter().find(|(_, t, a, _)| t == title && a == artist) {
        return Some((&v.0, 1));
    }

    // Tier 2: fuzzy title + author match, most viewed wins
    let nt = normalize(title);
    if nt.is_empty() {
        return None;
    }
    let candidates = artist_candidates(artist);

    let title_hit = |t: &str| {
        !normalize(t).is_empty() && (normalize(t).contains(&nt) || nt.contains(&normalize(t)))
    };
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
            mt.contains(c.as_str())
                || c.contains(mt.as_str())
                || mt_nospace.contains(&cn)
                || cn.contains(&mt_nospace)
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
    use super::client::collect_videos;
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
    }

    #[test]
    fn pick_tier4_rescues_top_when_title_mismatches_but_author_matches() {
        let videos = collect_videos(&fixture());
        assert_eq!(
            pick_video(&videos, "存在しないタイトル", "Artist A"),
            Some(("test0000001", 4))
        );
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
        let videos = vec![entry(
            "t1",
            "テスト楽曲 - テスト作者",
            "Test Author - Topic",
            10,
        )];
        assert!(pick_video(&videos, "テスト楽曲", "テスト作者").is_some());
    }

    #[test]
    fn pick_collab_jp_artist_matches_each_channel() {
        let a_first = vec![
            entry("v1", "テスト楽曲 (Music Video)", "テスト作者A", 500_000_000),
            entry(
                "v2",
                "テスト楽曲 (Official Audio)",
                "テスト作者B - Topic",
                100_000_000,
            ),
        ];
        assert_eq!(
            pick_video(&a_first, "テスト楽曲", "テスト作者A、テスト作者B"),
            Some(("v1", 2))
        );

        let b_first = vec![
            entry("v1", "テスト楽曲 (Music Video)", "テスト作者A", 100_000_000),
            entry(
                "v2",
                "テスト楽曲 (Official Audio)",
                "テスト作者B - Topic",
                500_000_000,
            ),
        ];
        assert_eq!(
            pick_video(&b_first, "テスト楽曲", "テスト作者A、テスト作者B"),
            Some(("v2", 2))
        );
    }

    #[test]
    fn pick_collab_en_artist_matches_topic_channel() {
        let videos = vec![
            entry("v1", "Collab Song (Music Video)", "Artist A", 100_000_000),
            entry(
                "v2",
                "Collab Song (Official Audio)",
                "Artist B - Topic",
                50_000_000,
            ),
        ];
        assert_eq!(
            pick_video(&videos, "Collab Song", "Artist A, Artist B"),
            Some(("v1", 2))
        );
    }

    #[test]
    fn pick_full_artist_string_survives_commas_in_name() {
        let videos = vec![entry(
            "ewf",
            "Test Song (Remastered)",
            "Test Artist, Test Group",
            900_000,
        )];
        assert_eq!(
            pick_video(&videos, "Test Song", "Test Artist, Test Group"),
            Some(("ewf", 2))
        );
    }

    fn entry(id: &str, title: &str, author: &str, views: u64) -> VideoEntry {
        (id.into(), title.into(), author.into(), views)
    }
}
