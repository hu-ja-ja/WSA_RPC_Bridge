use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub start_in_tray: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            start_in_tray: true,
            minimize_to_tray: true,
            close_to_tray: true,
        }
    }
}

fn config_path() -> PathBuf {
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join("wsa-rpc-bridge").join("config.json")
}

pub struct ConfigManager {
    inner: Mutex<Option<AppConfig>>,
    path: PathBuf,
}

impl ConfigManager {
    pub fn new() -> Self {
        let path = config_path();
        Self {
            inner: Mutex::new(None),
            path,
        }
    }

    pub fn get(&self) -> AppConfig {
        let mut guard = self.inner.lock().unwrap();
        if guard.is_none() {
            let config = fs::read_to_string(&self.path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            *guard = Some(config);
        }
        guard.as_ref().unwrap().clone()
    }

    pub fn set(&self, config: AppConfig) {
        let _ = fs::create_dir_all(self.path.parent().unwrap());
        let json = serde_json::to_string_pretty(&config).unwrap();
        let _ = fs::write(&self.path, &json);
        *self.inner.lock().unwrap() = Some(config);
    }
}
