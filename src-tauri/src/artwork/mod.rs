pub mod nicobox;

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use async_trait::async_trait;

use crate::models::MediaInfo;

const PLACEHOLDER_URL: &str = "https://placehold.co/100x100/000000/000000.png";

#[async_trait]
pub trait ArtworkResolver: Send + Sync {
    fn package_name(&self) -> &str;
    async fn resolve(&self, info: &MediaInfo, cache_dir: &Path, cache_enabled: bool) -> Option<String>;
}

pub struct ArtworkRegistry {
    resolvers: Vec<Box<dyn ArtworkResolver>>,
    cache_dir: PathBuf,
    in_memory: HashMap<(String, String, String), Option<String>>,
    cache_enabled: bool,
}

fn cache_filename(info: &MediaInfo) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (format!("{}{}{}", info.package_name, info.title, info.artist)).hash(&mut hasher);
    format!("{:x}.jpg", hasher.finish())
}

impl ArtworkRegistry {
    pub fn new(cache_dir: PathBuf, cache_enabled: bool) -> Self {
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            log::warn!("artwork: failed to create cache dir {:?}: {e}", cache_dir);
        }
        Self {
            resolvers: Vec::new(),
            cache_dir,
            in_memory: HashMap::new(),
            cache_enabled,
        }
    }

    pub fn register(&mut self, resolver: Box<dyn ArtworkResolver>) {
        self.resolvers.push(resolver);
    }

    pub fn update_cache_settings(&mut self, enabled: bool, cache_dir: Option<PathBuf>) {
        self.cache_enabled = enabled;
        if let Some(dir) = cache_dir {
            self.cache_dir = dir;
            let _ = std::fs::create_dir_all(&self.cache_dir);
        }
    }

    pub async fn resolve(&mut self, info: &MediaInfo) -> Option<String> {
        if self.cache_enabled {
            let key = (
                info.package_name.clone(),
                info.title.clone(),
                info.artist.clone(),
            );
            if let Some(cached) = self.in_memory.get(&key) {
                return cached.clone();
            }
        }

        let resolver = self
            .resolvers
            .iter()
            .find(|r| r.package_name() == info.package_name);

        let url = match resolver {
            Some(r) => r.resolve(info, &self.cache_dir, self.cache_enabled).await,
            None => None,
        };

        if let Some(ref u) = url {
            log::info!(
                "artwork: resolved for {} - {}",
                info.package_name,
                info.title
            );
            if self.cache_enabled {
                let key = (
                    info.package_name.clone(),
                    info.title.clone(),
                    info.artist.clone(),
                );
                self.in_memory.insert(key, Some(u.clone()));
            }
            return Some(u.clone());
        }

        log::debug!("artwork: no resolver found for {}, using placeholder", info.package_name);
        if self.cache_enabled {
            let key = (
                info.package_name.clone(),
                info.title.clone(),
                info.artist.clone(),
            );
            self.in_memory.insert(key, Some(PLACEHOLDER_URL.to_string()));
        }
        Some(PLACEHOLDER_URL.to_string())
    }

}
