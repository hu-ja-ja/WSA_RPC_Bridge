use std::collections::HashMap;
use std::sync::OnceLock;

use jni::jni_sig;
use jni::jni_str;
use tauri::{AppHandle, Emitter};

static JVM: OnceLock<jni::JavaVM> = OnceLock::new();

static BRIDGE_CLASS: OnceLock<jni::objects::Global<jni::objects::JClass<'static>>> =
    OnceLock::new();

#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_UpdateBridge_init(
    mut unowned_env: jni::EnvUnowned,
    _this: jni::objects::JObject,
) {
    unowned_env
        .with_env(|env| -> jni::errors::Result<()> {
            if let Ok(vm) = env.get_java_vm() {
                let _ = JVM.set(vm);
            }
            // JNI の FindClass はメインスレッド以外ではアプリのクラスローダーを参照しないため、
            // メインスレッドでグローバル参照としてクラスを取得し、以降はそれを使う。
            if let Ok(class) = env.find_class(jni_str!("com/wsarpcbridge/app/UpdateBridge")) {
                if let Ok(gref) = env.new_global_ref(class) {
                    let _ = BRIDGE_CLASS.set(gref);
                }
            }
            Ok(())
        })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

fn with_jni<T>(f: impl FnOnce(&mut jni::Env) -> jni::errors::Result<T>) -> Result<T, String> {
    let vm = JVM
        .get()
        .ok_or("JVM not initialized (UpdateBridge.init not called)")?;
    vm.attach_current_thread(|env| {
        let result = f(env);
        if result.is_err() {
            let _ = env.exception_clear();
        }
        result
    })
    .map_err(|e| e.to_string())
}

fn bridge_class() -> Result<&'static jni::objects::Global<jni::objects::JClass<'static>>, String> {
    BRIDGE_CLASS
        .get()
        .ok_or_else(|| "UpdateBridge class not cached (UpdateBridge.init not called)".to_string())
}

/// update.json の取得先。Win版 updater の endpoints と同じものを見る。
const UPDATE_URL: &str = "https://hu-ja-ja.github.io/WSA_RPC_Bridge/update.json";
/// update.json の platforms 内の Android 用キー。release.yml で付与する。
const PLATFORM_KEY: &str = "android-aarch64";

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidUpdateStatus {
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    pub url: Option<String>,
}

#[derive(serde::Deserialize)]
struct UpdateManifest {
    version: Option<String>,
    platforms: Option<HashMap<String, PlatformEntry>>,
}

#[derive(serde::Deserialize)]
struct PlatformEntry {
    url: Option<String>,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    progress: u32,
}

/// 先頭の空白・v を除き、数値部分だけを比較する。欠けた桁は 0 扱い。
// ponytail: semver クレート1つのために依存を増やさない
pub(crate) fn is_newer(current: &str, latest: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        let v = v.trim().trim_start_matches(['v', 'V']);
        let core = v.split(['-', '+']).next().unwrap_or("");
        core.split('.')
            .map(|s| {
                s.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse::<u64>()
                    .unwrap_or(0)
            })
            .collect()
    }
    let (mut a, mut b) = (parts(current), parts(latest));
    let n = a.len().max(b.len()).max(1);
    a.resize(n, 0);
    b.resize(n, 0);
    if a == b {
        // 数値部が同じならプレリリース接尾辞の有無だけ見る (1.0.0-beta < 1.0.0)
        return current.contains('-') && !latest.contains('-');
    }
    b > a
}

pub async fn fetch_update_status(current: &str) -> Result<AndroidUpdateStatus, String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|e| e.to_string())?;
    let manifest: UpdateManifest = client
        .get(UPDATE_URL)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let latest = manifest.version.unwrap_or_default();
    let url = manifest
        .platforms
        .and_then(|p| p.get(PLATFORM_KEY).and_then(|e| e.url.clone()));
    Ok(AndroidUpdateStatus {
        current_version: current.to_string(),
        latest_version: latest.clone(),
        available: !latest.is_empty() && is_newer(current, &latest),
        url,
    })
}

fn cache_dir() -> Result<String, String> {
    let class = bridge_class()?;
    let dir: String = with_jni(|env| {
        let result = env.call_static_method(
            class,
            jni_str!("cacheDirPath"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?;
        let jstr: jni::objects::JString = env.cast_local::<jni::objects::JString>(result.l()?)?;
        jstr.try_to_string(env)
    })?;
    if dir.is_empty() {
        return Err("cache directory unavailable".to_string());
    }
    Ok(dir)
}

fn sanitize_version(version: &str) -> String {
    // ponytail: ファイル名に使えない文字を落とす
    version
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-' || *c == '_')
        .collect()
}

fn apk_path_for(version: &str) -> Result<String, String> {
    let safe = sanitize_version(version);
    if safe.is_empty() {
        return Err("invalid version for apk path".to_string());
    }
    Ok(format!("{}/update-{}.apk", cache_dir()?, safe))
}

/// APKをキャッシュに保存する。同版が済みなら再取得しない。進捗はイベントで通知する。
/// 破損・中断時は .part を残さず、content-length と突き合わせてから rename する。
pub async fn download_update(app: &AppHandle, url: &str, version: &str) -> Result<String, String> {
    if !url.starts_with("https://") {
        return Err("update url must be https".to_string());
    }
    let path = apk_path_for(version)?;
    if std::path::Path::new(&path).is_file()
        && std::fs::metadata(&path)
            .map(|m| m.len() > 0)
            .unwrap_or(false)
    {
        return Ok(path);
    }
    let tmp = format!("{}.part", &path);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;
    let result: Result<String, String> = async {
        let mut resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let total = resp.content_length().filter(|t| *t > 0);
        let mut file = tokio::fs::File::create(&tmp)
            .await
            .map_err(|e| e.to_string())?;
        let mut done: u64 = 0;
        {
            use tokio::io::AsyncWriteExt;
            while let Some(chunk) = resp.chunk().await.map_err(|e| e.to_string())? {
                file.write_all(&chunk).await.map_err(|e| e.to_string())?;
                done += chunk.len() as u64;
                if let Some(t) = total {
                    let _ = app.emit(
                        "android-update-progress",
                        DownloadProgress {
                            progress: (done.saturating_mul(100) / t).min(100) as u32,
                        },
                    );
                }
            }
            file.flush().await.map_err(|e| e.to_string())?;
        }
        drop(file);
        if let Some(t) = total {
            if done != t {
                return Err("incomplete download, please retry".to_string());
            }
        }
        tokio::fs::rename(&tmp, &path)
            .await
            .map_err(|e| e.to_string())?;
        Ok(path.clone())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&tmp).await;
    }
    result
}

pub fn can_install_packages() -> Result<bool, String> {
    let class = bridge_class()?;
    with_jni(|env| {
        let result =
            env.call_static_method(class, jni_str!("canRequestInstalls"), jni_sig!("()Z"), &[])?;
        result.z()
    })
}

pub fn open_install_settings() -> Result<(), String> {
    let class = bridge_class()?;
    with_jni(|env| {
        env.call_static_method(class, jni_str!("openInstallSettings"), jni_sig!("()V"), &[])?;
        Ok(())
    })
}

pub fn install_apk(path: &str) -> Result<(), String> {
    let cache = cache_dir()?;
    if !path.ends_with(".apk") || !path.starts_with(&cache) {
        return Err("invalid apk path".to_string());
    }
    if !std::path::Path::new(path).is_file() {
        return Err("apk cache missing, please re-download".to_string());
    }
    let class = bridge_class()?;
    let launched = with_jni(|env| {
        let jpath = env.new_string(path)?;
        let result = env.call_static_method(
            class,
            jni_str!("installApk"),
            jni_sig!("(Ljava/lang/String;)Z"),
            &[(&jpath).into()],
        )?;
        result.z()
    })?;
    if launched {
        Ok(())
    } else {
        log::warn!("install failed: path exists but installer not launched");
        Err("installer could not be launched".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{is_newer, sanitize_version};

    #[test]
    fn version_compare() {
        assert!(is_newer("0.4.0", "0.4.1"));
        assert!(is_newer("0.4.0", "0.5.0"));
        assert!(is_newer("0.4.0", "1.0.0"));
        assert!(!is_newer("0.4.1", "0.4.1"));
        assert!(!is_newer("0.5.0", "0.4.9"));
        assert!(!is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.4", "0.4.1"));
        assert!(!is_newer("0.4.1", "0.4"));
        assert!(is_newer("v0.4.0", "0.4.1"));
        assert!(is_newer("0.4.0", "v0.4.1"));
        assert!(is_newer("1.0.0-beta", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.0-beta"));
        // ponytail: beta同士は比較しない仕様
        assert!(!is_newer("1.0.0-beta.1", "1.0.0-beta.2"));
    }

    #[test]
    fn version_sanitize() {
        assert_eq!(sanitize_version("0.4.1"), "0.4.1");
        assert_eq!(sanitize_version("../0.4"), "0.4");
        assert_eq!(sanitize_version(""), "");
        assert_eq!(sanitize_version("../../etc"), "etc");
    }
}
