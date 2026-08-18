use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};

use tauri::{AppHandle, Emitter, Manager};

use crate::android::discord::{discord_connect, discord_disconnect, update_presence_dedup};
use crate::commands::AppState;
use crate::models::MediaInfo;

static MEDIA_STATE: OnceLock<Mutex<MediaInfo>> = OnceLock::new();

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

pub fn set_app_handle(app: AppHandle) {
    let _ = APP_HANDLE.set(app);
}

pub fn app_handle() -> Option<AppHandle> {
    APP_HANDLE.get().cloned()
}

pub fn media_state() -> &'static Mutex<MediaInfo> {
    MEDIA_STATE.get_or_init(|| Mutex::new(MediaInfo::default()))
}

// ---- ホワイトリスト (Kotlin の MediaWhitelistStore へ JNI で橋渡し) ----

static JVM: OnceLock<jni::JavaVM> = OnceLock::new();

static STORE_CLASS: OnceLock<jni::objects::GlobalRef> = OnceLock::new();

static INFO_SERVICE_CLASS: OnceLock<jni::objects::GlobalRef> = OnceLock::new();

#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_MediaBridge_init(
    mut env: jni::JNIEnv,
    _this: jni::objects::JObject,
) {
    if let Ok(vm) = env.get_java_vm() {
        let _ = JVM.set(vm);
    }
    // JNI の FindClass はメインスレッド以外ではアプリのクラスローダーを参照しないため、
    // メインスレッドでグローバル参照としてクラスを取得し、以降はそれを使う。
    if let Ok(class) = env.find_class("com/wsarpcbridge/app/MediaWhitelistStore") {
        if let Ok(gref) = env.new_global_ref(class) {
            let _ = STORE_CLASS.set(gref);
        }
    }
    if let Ok(class) = env.find_class("com/wsarpcbridge/app/MediaInfoService") {
        if let Ok(gref) = env.new_global_ref(class) {
            let _ = INFO_SERVICE_CLASS.set(gref);
        }
    }
}

fn with_jni<T>(f: impl FnOnce(&mut jni::JNIEnv) -> jni::errors::Result<T>) -> Result<T, String> {
    let vm = JVM.get().ok_or("JVM not initialized (MediaBridge.init not called)")?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    match f(&mut env) {
        Ok(v) => Ok(v),
        Err(e) => {
            // 失敗時は pending exception を残さない。残すと次の JNI 呼び出しで
            // "unexpected pending exception" により ART がプロセスごと abort する。
            let _ = env.exception_clear();
            Err(e.to_string())
        }
    }
}

fn jstring_array_to_vec(
    env: &mut jni::JNIEnv,
    arr: jni::objects::JObject,
) -> jni::errors::Result<Vec<String>> {
    let arr = jni::objects::JObjectArray::from(arr);
    let len = env.get_array_length(&arr)?;
    let mut out = Vec::with_capacity(len as usize);
    for i in 0..len {
        let el = env.get_object_array_element(&arr, i)?;
        out.push(env.get_string(&jni::objects::JString::from(el))?.into());
    }
    Ok(out)
}

fn call_string_array(name: &str) -> Result<Vec<String>, String> {
    let class = store_class()?;
    with_jni(|env| {
        let result = env.call_static_method(class, name, "()[Ljava/lang/String;", &[])?;
        jstring_array_to_vec(env, result.l()?)
    })
}

fn store_class() -> Result<&'static jni::objects::GlobalRef, String> {
    STORE_CLASS
        .get()
        .ok_or_else(|| "MediaWhitelistStore class not cached (MediaBridge.init not called)".to_string())
}

fn info_service_class() -> Result<&'static jni::objects::GlobalRef, String> {
    INFO_SERVICE_CLASS
        .get()
        .ok_or_else(|| "MediaInfoService class not cached (MediaBridge.init not called)".to_string())
}

pub fn load_media_notification_enabled() -> Result<bool, String> {
    let class = info_service_class()?;
    with_jni(|env| {
        let result = env.call_static_method(class, "isEnabled", "()Z", &[])?;
        result.z()
    })
}

pub fn set_media_notification_enabled(enabled: bool) -> Result<(), String> {
    let class = info_service_class()?;
    with_jni(|env| {
        env.call_static_method(class, "setEnabled", "(Z)V", &[jni::objects::JValue::Bool(enabled as jni::sys::jboolean)])?;
        Ok(())
    })
}

pub fn load_rpc_enabled() -> Result<bool, String> {
    let class = info_service_class()?;
    with_jni(|env| {
        let result = env.call_static_method(class, "isRpcEnabled", "()Z", &[])?;
        result.z()
    })
}

fn set_media_rpc_enabled(enabled: bool) -> Result<(), String> {
    let class = info_service_class()?;
    with_jni(|env| {
        env.call_static_method(class, "setRpcEnabled", "(Z)V", &[jni::objects::JValue::Bool(enabled as jni::sys::jboolean)])?;
        Ok(())
    })
}

/// RPC ON/OFF を永続化し、Discord 接続を切替えてイベントを発行する。
/// 通知ボタン(JNI)と WebView(コマンド)の両方から呼ばれる。
pub fn set_rpc_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    set_media_rpc_enabled(enabled)?;
    let state = app.state::<AppState>();
    if enabled {
        if crate::android::rpc_idle() {
            log::warn!("android: rpc enable skipped (idle disconnect active)");
            state.discord_connected.store(false, Ordering::Relaxed);
        } else {
            discord_connect()?;
            let info = media_state().lock().expect("media mutex poisoned").clone();
            if !info.title.is_empty() {
                if let Err(e) = update_presence_dedup(&info) {
                    log::warn!("android: presence push after rpc enable failed: {e}");
                }
            }
            state.discord_connected.store(true, Ordering::Relaxed);
        }
    } else {
        discord_disconnect()?;
        state.discord_connected.store(false, Ordering::Relaxed);
    }
    let _ = app.emit("discord-status-changed", enabled);
    let _ = app.emit("rpc-enabled-changed", enabled);
    Ok(())
}

pub fn list_media_apps() -> Result<Vec<String>, String> {
    call_string_array("listApps")
}

pub fn load_whitelist() -> Result<Vec<String>, String> {
    call_string_array("load")
}

pub fn save_whitelist(packages: &[String]) -> Result<(), String> {
    let class = store_class()?;
    with_jni(|env| {
        let arr = env.new_object_array(packages.len() as i32, "java/lang/String", jni::objects::JObject::null())?;
        for (i, pkg) in packages.iter().enumerate() {
            let js = env.new_string(pkg.as_str())?;
            env.set_object_array_element(&arr, i as i32, js)?;
        }
        let _ = env.call_static_method(class, "save", "([Ljava/lang/String;)V", &[jni::objects::JValue::from(&arr)])?;
        Ok(())
    })
}

#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_MediaBridge_updateMediaInfo(
    mut env: jni::JNIEnv,
    _this: jni::objects::JObject,
    title: jni::objects::JString,
    artist: jni::objects::JString,
    album: jni::objects::JString,
    package_name: jni::objects::JString,
    display_name: jni::objects::JString,
    position_ms: jni::sys::jlong,
    duration_ms: jni::sys::jlong,
    is_playing: jni::sys::jboolean,
) {
    let mut get = |s: &jni::objects::JString| env.get_string(s).map(|v| v.into()).unwrap_or_default();
    let display_name: String = get(&display_name);
    let display_name = (!display_name.is_empty()).then_some(display_name);

    let info = MediaInfo {
        title: get(&title),
        artist: get(&artist),
        album: get(&album),
        package_name: get(&package_name),
        thumbnail_url: None,
        position: (position_ms > 0).then_some(position_ms as u64),
        duration: (duration_ms > 0).then_some(duration_ms as u64),
        display_name,
        is_playing: is_playing != 0,
    };

    log::debug!("android: media update: {} - {} (playing={})", info.title, info.artist, info.is_playing);
    *media_state().lock().expect("media mutex poisoned") = info.clone();

    let Some(app) = APP_HANDLE.get().cloned() else {
        return;
    };
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        let mut info = info;
        if let Some(url) = state.artwork.lock().await.resolve(&info).await {
            info.thumbnail_url = Some(url);
        }
        let _ = app.emit("media-updated", &info);

        if info.title.is_empty() {
            // ホワイトリスト対象のメディアセッションが消えた → RPCを切断する
            if state.discord_connected.swap(false, Ordering::Relaxed) {
                if let Err(e) = discord_disconnect() {
                    log::warn!("android: session-lost disconnect failed: {e}");
                }
                let _ = app.emit("discord-status-changed", false);
            }
            return;
        }
        if !state.discord_connected.load(Ordering::Relaxed) {
            log::debug!("android: presence update skipped (not connected)");
            return;
        }
        if let Err(e) = update_presence_dedup(&info) {
            log::warn!("android: presence update failed: {e}");
        }
    });
}

/// 通知ボタンから呼ばれる。永続化は Kotlin 側(MediaInfoService.setRpcEnabled)で済んでいるため、
/// 接続制御とイベント発行のみ行う。
#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_MediaBridge_setRpcEnabled(
    _env: jni::JNIEnv,
    _this: jni::objects::JObject,
    enabled: jni::sys::jboolean,
) {
    let Some(app) = APP_HANDLE.get().cloned() else {
        return;
    };
    if let Err(e) = set_rpc_enabled(&app, enabled != 0) {
        log::error!("android: setRpcEnabled JNI failed: {e}");
    }
}