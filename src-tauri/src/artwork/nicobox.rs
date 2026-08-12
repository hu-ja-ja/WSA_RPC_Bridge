use async_trait::async_trait;
use reqwest::Client;

use crate::artwork::ArtworkResolver;
use crate::models::MediaInfo;

pub struct NicoboxResolver {
    client: Client,
}

impl NicoboxResolver {
    pub fn new() -> Self {
        Self { client: Client::new() }
    }
}

#[async_trait]
impl ArtworkResolver for NicoboxResolver {
    fn package_name(&self) -> &str {
        "jp.nicovideo.nicobox"
    }

    async fn resolve(&self, info: &MediaInfo) -> Option<String> {
        let query = format!("\"{}\"", info.title);
        let encoded = urlencoding::encode(&query);

        let url = format!(
            "https://snapshot.search.nicovideo.jp/api/v2/snapshot/video/contents/search?\
             q={encoded}&targets=title&fields=contentId,title,thumbnailUrl&\
             _sort=-viewCounter&_offset=0&_limit=1&_context=wsa_rpc_bridge"
        );

        log::debug!("nicobox: searching niconico API: {}", url);

        let resp = self
            .client
            .get(&url)
            .header("User-Agent", concat!("wsa_rpc_bridge/", env!("CARGO_PKG_VERSION")))
            .send()
            .await
            .ok()?;

        let status = resp.status();
        if !status.is_success() {
            log::warn!("nicobox: API returned {}", status);
            return None;
        }

        let json: serde_json::Value = resp.json().await.ok()?;

        let data = &json["data"][0];
        let title = data["title"].as_str()?;
        if title != info.title {
            log::debug!("nicobox: title mismatch: got \"{title}\", expected \"{}\"", info.title);
            return None;
        }

        let thumbnail_url = data["thumbnailUrl"].as_str()?.to_string();

        log::info!("nicobox: resolved thumbnail: {}", thumbnail_url);
        Some(thumbnail_url)
    }
}
