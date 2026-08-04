use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub auto_start: bool,
    pub start_in_tray: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub thumbnail_cache_enabled: bool,
    pub thumbnail_cache_path: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            auto_start: false,
            start_in_tray: true,
            minimize_to_tray: true,
            close_to_tray: true,
            thumbnail_cache_enabled: true,
            thumbnail_cache_path: None,
        }
    }
}

pub(crate) fn app_data_base(env_var: &str, fallback_dir: &str) -> PathBuf {
    std::env::var(env_var)
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join("AppData").join(fallback_dir)
        })
}

fn config_path() -> PathBuf {
    app_data_base("APPDATA", "Roaming")
        .join("wsa-rpc-bridge")
        .join("config.json")
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
        let mut guard = self.inner.lock().expect("config mutex poisoned");
        if guard.is_none() {
            let config = match fs::read_to_string(&self.path) {
                Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
                Err(_) => {
                    let default = AppConfig::default();
                    if let Some(parent) = self.path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    if let Ok(json) = serde_json::to_string_pretty(&default) {
                        let _ = fs::write(&self.path, &json);
                        log::info!("config file created with defaults");
                    }
                    default
                }
            };
            *guard = Some(config);
        }
        guard.as_ref().expect("config not initialized").clone()
    }

    pub fn set(&self, config: AppConfig) {
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&config) {
            let _ = fs::write(&self.path, &json);
        }
        *self.inner.lock().expect("config mutex poisoned") = Some(config);
    }
}
