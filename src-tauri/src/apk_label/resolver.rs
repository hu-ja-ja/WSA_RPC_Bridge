use std::collections::HashMap;
use std::path::{Path, PathBuf};

use adb_client::ADBDeviceExt;
use anyhow::Context;
use anyhow::Result;

pub struct ApkLabelResolver {
    cache: HashMap<String, String>,
    cache_dir: PathBuf,
}

impl ApkLabelResolver {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache: HashMap::new(),
            cache_dir,
        }
    }

    pub async fn resolve<D: ADBDeviceExt + Send>(&mut self, package_name: &str, device: &mut D) -> String {
        if let Some(name) = self.cache.get(package_name) {
            return name.clone();
        }

        match self.resolve_inner(package_name, device).await {
            Ok(name) => {
                log::info!("Resolved app label: {} -> {}", package_name, name);
                self.cache.insert(package_name.to_string(), name.clone());
                name
            }
            Err(e) => {
                log::warn!("Failed to resolve app label for {}: {:#}", package_name, e);
                package_name.to_string()
            }
        }
    }

    async fn resolve_inner<D: ADBDeviceExt + Send>(&self, package_name: &str, device: &mut D) -> Result<String> {
        let apk_path = self.get_apk_path(package_name, device).await?;
        let local_path = self.cache_dir.join(format!("{}.apk", package_name));

        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        self.pull_apk(&apk_path, &local_path, device).await?;
        let label = self.parse_apk_label(&local_path).await?;

        let _ = tokio::fs::remove_file(&local_path).await;

        if label.is_empty() {
            anyhow::bail!("empty application label in APK");
        }

        Ok(label)
    }

    async fn get_apk_path<D: ADBDeviceExt + Send>(&self, package_name: &str, device: &mut D) -> Result<String> {
        let cmd = format!("pm path {}", package_name);
        let mut stdout = Vec::new();
        device
            .shell_command(&cmd, Some(&mut stdout), None)
            .context("pm path failed")?;

        let stdout = String::from_utf8(stdout)?;

        let apk_path = stdout
            .lines()
            .filter_map(|line| {
                let path = line.trim().strip_prefix("package:")?.trim().to_string();
                if path.ends_with("/base.apk") { Some(path) } else { None }
            })
            .next()
            .ok_or_else(|| anyhow::anyhow!("no base.apk in pm path output"))?;

        log::debug!("APK path: {}", apk_path);
        Ok(apk_path)
    }

    async fn pull_apk<D: ADBDeviceExt + Send>(
        &self,
        remote_path: &str,
        local_path: &Path,
        device: &mut D,
    ) -> Result<()> {
        let remote_path = remote_path.to_string();
        let local_path = local_path.to_path_buf();
        let mut file = std::fs::File::create(&local_path)?;
        device
            .pull(&remote_path, &mut file)
            .context("adb pull failed")?;

        log::debug!("APK pulled to {:?}", local_path);
        Ok(())
    }

    async fn parse_apk_label(&self, path: &Path) -> Result<String> {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let apk = apk_info::Apk::new(&path)?;
            let label = apk.get_application_label().unwrap_or_default();
            if label.is_empty() {
                anyhow::bail!("empty application label in APK");
            }
            Ok(label)
        })
        .await?
    }
}
