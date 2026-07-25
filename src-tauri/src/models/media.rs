use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub package_name: String,
    pub thumbnail_url: Option<String>,
    pub position: Option<u64>,
    pub duration: Option<u64>,
    pub is_playing: bool,
}
