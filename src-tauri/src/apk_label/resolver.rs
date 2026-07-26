use std::collections::HashMap;
use std::path::PathBuf;

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

    pub async fn resolve(&mut self, package_name: &str) -> String {
        if let Some(name) = self.cache.get(package_name) {
            return name.clone();
        }

        match self.resolve_inner(package_name).await {
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

    async fn resolve_inner(&self, package_name: &str) -> Result<String> {
        let apk_path = self.get_apk_path(package_name).await?;
        let local_path = self.cache_dir.join(format!("{}.apk", package_name));

        if let Some(parent) = local_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        self.pull_apk(&apk_path, &local_path).await?;
        let label = self.parse_apk_label(&local_path).await?;

        let _ = tokio::fs::remove_file(&local_path).await;

        if label.is_empty() {
            anyhow::bail!("empty application label in APK");
        }

        Ok(label)
    }

    async fn get_apk_path(&self, package_name: &str) -> Result<String> {
        let output = tokio::process::Command::new("adb")
            .args(["shell", "pm", "path", package_name])
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!(
                "pm path failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        let stdout = String::from_utf8(output.stdout)?;

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

    async fn pull_apk(&self, remote_path: &str, local_path: &PathBuf) -> Result<()> {
        let output = tokio::process::Command::new("adb")
            .args(["pull", remote_path])
            .arg(local_path)
            .output()
            .await?;

        if !output.status.success() {
            anyhow::bail!(
                "adb pull failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        log::debug!("APK pulled to {:?}", local_path);
        Ok(())
    }

    async fn parse_apk_label(&self, path: &PathBuf) -> Result<String> {
        let path = path.clone();
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
