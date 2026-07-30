use std::path::{Path, PathBuf};

use async_trait::async_trait;
use reqwest::Client;

use crate::artwork::{cache_filename, ArtworkResolver};
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

    async fn resolve(&self, info: &MediaInfo, cache_dir: &Path, cache_enabled: bool) -> Option<String> {
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
            .header("User-Agent", "wsa_rpc_bridge/0.2.0")
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

        if cache_enabled {
            let local_path = cache_dir.join(cache_filename(info));
            if !local_path.exists() {
                if let Err(e) = download_image(&self.client, &thumbnail_url, &local_path).await {
                    log::warn!("nicobox: failed to cache thumbnail: {e}");
                }
            }
        }

        log::info!("nicobox: resolved thumbnail: {}", thumbnail_url);
        Some(thumbnail_url)
    }
}

async fn download_image(client: &Client, url: &str, path: &PathBuf) -> Result<(), String> {
    let bytes = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("request failed: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("read body failed: {e}"))?;

    tokio::fs::write(path, &bytes)
        .await
        .map_err(|e| format!("write failed: {e}"))?;

    log::debug!("nicobox: cached thumbnail to {:?}", path);
    Ok(())
}
