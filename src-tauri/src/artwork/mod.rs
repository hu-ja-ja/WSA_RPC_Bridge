pub mod nicobox;

use std::collections::HashMap;

use async_trait::async_trait;

use crate::models::MediaInfo;

const PLACEHOLDER_URL: &str = "https://placehold.co/100x100/000000/000000.png";

#[async_trait]
pub trait ArtworkResolver: Send + Sync {
    fn package_name(&self) -> &str;
    async fn resolve(&self, info: &MediaInfo) -> Option<String>;
}

pub struct ArtworkRegistry {
    resolvers: Vec<Box<dyn ArtworkResolver>>,
    in_memory: HashMap<(String, String, String), Option<String>>,
}

impl ArtworkRegistry {
    pub fn new() -> Self {
        Self {
            resolvers: Vec::new(),
            in_memory: HashMap::new(),
        }
    }

    pub fn register(&mut self, resolver: Box<dyn ArtworkResolver>) {
        self.resolvers.push(resolver);
    }

    pub async fn resolve(&mut self, info: &MediaInfo) -> Option<String> {
        let key = (
            info.package_name.clone(),
            info.title.clone(),
            info.artist.clone(),
        );
        if let Some(cached) = self.in_memory.get(&key) {
            return cached.clone();
        }

        let resolver = self
            .resolvers
            .iter()
            .find(|r| r.package_name() == info.package_name);

        let url = match resolver {
            Some(r) => r.resolve(info).await,
            None => None,
        };

        if let Some(ref u) = url {
            log::info!(
                "artwork: resolved for {} - {}",
                info.package_name,
                info.title
            );
            self.in_memory.insert(key, Some(u.clone()));
            return Some(u.clone());
        }

        log::debug!("artwork: no resolver found for {}, using placeholder", info.package_name);
        self.in_memory.insert(key, Some(PLACEHOLDER_URL.to_string()));
        Some(PLACEHOLDER_URL.to_string())
    }
}
