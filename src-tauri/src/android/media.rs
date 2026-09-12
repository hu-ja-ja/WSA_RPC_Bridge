use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};

use jni::jni_sig;
use jni::jni_str;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::android::discord::{
    discord_connect, discord_disconnect, discord_update_presence, update_presence_dedup,
};
use crate::commands::AppState;
use crate::models::MediaInfo;

#[derive(Clone, Serialize)]
struct ThumbnailPayload {
    package_name: String,
    title: String,
    artist: String,
    thumbnail_url: String,
}

static MEDIA_STATE: OnceLock<Mutex<MediaInfo>> = OnceLock::new();

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();

// AppHandle 未設定中(サービスが MainActivity より先に起動)に受けた最新の更新を1件だけ保留する。
static PENDING_MEDIA: OnceLock<Mutex<Option<MediaInfo>>> = OnceLock::new();

fn pending_media() -> &'static Mutex<Option<MediaInfo>> {
    PENDING_MEDIA.get_or_init(|| Mutex::new(None))
}

pub fn set_app_handle(app: AppHandle) {
    let _ = APP_HANDLE.set(app.clone());
    if let Some(info) = pending_media()
        .lock()
        .expect("pending media mutex poisoned")
        .take()
    {
        push_media_update(app, info);
    }
}

pub fn app_handle() -> Option<AppHandle> {
    APP_HANDLE.get().cloned()
}

pub fn media_state() -> &'static Mutex<MediaInfo> {
    MEDIA_STATE.get_or_init(|| Mutex::new(MediaInfo::default()))
}

// ---- ホワイトリスト (Kotlin の MediaWhitelistStore へ JNI で橋渡し) ----

static JVM: OnceLock<jni::JavaVM> = OnceLock::new();

static STORE_CLASS: OnceLock<jni::objects::Global<jni::objects::JClass<'static>>> = OnceLock::new();

static INFO_SERVICE_CLASS: OnceLock<jni::objects::Global<jni::objects::JClass<'static>>> =
    OnceLock::new();

#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_MediaBridge_init(
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
            if let Ok(class) = env.find_class(jni_str!("com/wsarpcbridge/app/MediaWhitelistStore"))
            {
                if let Ok(gref) = env.new_global_ref(class) {
                    let _ = STORE_CLASS.set(gref);
                }
            }
            if let Ok(class) = env.find_class(jni_str!("com/wsarpcbridge/app/MediaInfoService")) {
                if let Ok(gref) = env.new_global_ref(class) {
                    let _ = INFO_SERVICE_CLASS.set(gref);
                }
            }
            Ok(())
        })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();
}

fn with_jni<T>(f: impl FnOnce(&mut jni::Env) -> jni::errors::Result<T>) -> Result<T, String> {
    let vm = JVM
        .get()
        .ok_or("JVM not initialized (MediaBridge.init not called)")?;
    vm.attach_current_thread(|env| {
        let result = f(env);
        if result.is_err() {
            // 失敗時は pending exception を残さない。残すと次の JNI 呼び出しで
            // "unexpected pending exception" により ART がプロセスごと abort する。
            let _ = env.exception_clear();
        }
        result
    })
    .map_err(|e| e.to_string())
}

fn jstring_array_to_vec(
    env: &mut jni::Env,
    arr: jni::objects::JObject,
) -> jni::errors::Result<Vec<String>> {
    let arr = env.cast_local::<jni::objects::JObjectArray<jni::objects::JString>>(arr)?;
    let len = arr.len(env)?;
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        let el: jni::objects::JString = arr.get_element(env, i)?;
        out.push(el.try_to_string(env)?);
    }
    Ok(out)
}

fn call_string_array(name: &str) -> Result<Vec<String>, String> {
    let class = store_class()?;
    let name = jni::strings::JNIString::from(name);
    with_jni(|env| {
        let result =
            env.call_static_method(class, &name, jni_sig!("()[Ljava/lang/String;"), &[])?;
        jstring_array_to_vec(env, result.l()?)
    })
}

fn store_class() -> Result<&'static jni::objects::Global<jni::objects::JClass<'static>>, String> {
    STORE_CLASS.get().ok_or_else(|| {
        "MediaWhitelistStore class not cached (MediaBridge.init not called)".to_string()
    })
}

fn info_service_class(
) -> Result<&'static jni::objects::Global<jni::objects::JClass<'static>>, String> {
    INFO_SERVICE_CLASS.get().ok_or_else(|| {
        "MediaInfoService class not cached (MediaBridge.init not called)".to_string()
    })
}

pub fn load_media_notification_enabled() -> Result<bool, String> {
    let class = info_service_class()?;
    with_jni(|env| {
        let result = env.call_static_method(class, jni_str!("isEnabled"), jni_sig!("()Z"), &[])?;
        result.z()
    })
}

pub fn set_media_notification_enabled(enabled: bool) -> Result<(), String> {
    let class = info_service_class()?;
    with_jni(|env| {
        env.call_static_method(
            class,
            jni_str!("setEnabled"),
            jni_sig!("(Z)V"),
            &[jni::objects::JValue::Bool(enabled)],
        )?;
        Ok(())
    })
}

pub fn load_rpc_enabled() -> Result<bool, String> {
    let class = info_service_class()?;
    with_jni(|env| {
        let result =
            env.call_static_method(class, jni_str!("isRpcEnabled"), jni_sig!("()Z"), &[])?;
        result.z()
    })
}

fn set_media_rpc_enabled(enabled: bool) -> Result<(), String> {
    let class = info_service_class()?;
    with_jni(|env| {
        env.call_static_method(
            class,
            jni_str!("setRpcEnabled"),
            jni_sig!("(Z)V"),
            &[jni::objects::JValue::Bool(enabled)],
        )?;
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
    let _ = app.emit(
        "discord-status-changed",
        state.discord_connected.load(Ordering::Relaxed),
    );
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
        let empty = env.new_string("")?;
        let arr =
            jni::objects::JObjectArray::<jni::objects::JString>::new(env, packages.len(), &empty)?;
        for (i, pkg) in packages.iter().enumerate() {
            let js = env.new_string(pkg.as_str())?;
            arr.set_element(env, i, &js)?;
        }
        let _ = env.call_static_method(
            class,
            jni_str!("save"),
            jni_sig!("([Ljava/lang/String;)V"),
            &[jni::objects::JValue::from(&arr)],
        )?;
        Ok(())
    })
}

#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_MediaBridge_updateMediaInfo(
    mut unowned_env: jni::EnvUnowned,
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
    let info = unowned_env
        .with_env(|env| -> jni::errors::Result<MediaInfo> {
            let get = |s: &jni::objects::JString| s.try_to_string(env).unwrap_or_default();
            let display_name: String = get(&display_name);
            let display_name = (!display_name.is_empty()).then_some(display_name);

            Ok(MediaInfo {
                title: get(&title),
                artist: get(&artist),
                album: get(&album),
                package_name: get(&package_name),
                thumbnail_url: None,
                position: (position_ms > 0).then_some(position_ms as u64),
                duration: (duration_ms > 0).then_some(duration_ms as u64),
                display_name,
                is_playing,
            })
        })
        .resolve::<jni::errors::ThrowRuntimeExAndDefault>();

    crate::vlog!(
        "android: media update: title={} artist={} pkg={:?} playing={}",
        m(&info.title),
        m(&info.artist),
        info.package_name,
        info.is_playing
    );
    // ponytail: 情報とサムネは完全分離 — media_state はサムネを持たない
    *media_state().lock().expect("media mutex poisoned") = info.clone();

    let Some(app) = APP_HANDLE.get().cloned() else {
        // サービスが MainActivity より先に起動した場合(AppHandle 未設定)は最新の1件だけ保留し、
        // set_app_handle でリプレイする。UI側は起動時に get_media_info で現在値を取得する。
        *pending_media()
            .lock()
            .expect("pending media mutex poisoned") = Some(info);
        return;
    };
    push_media_update(app, info);
}

fn push_media_update(app: AppHandle, info: MediaInfo) {
    if cfg!(debug_assertions) {
        crate::mlog!(
            info,
            "android: push_media_update: title={} artist={} pkg={:?} playing={}",
            m(&info.title),
            m(&info.artist),
            info.package_name,
            info.is_playing
        );
    }
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();

        // ponytail: 情報とサムネ完全分離 — 情報は即時、サムネは別イベント
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

        // 曲切替時はDiscord即時送信（サムネ無しでOK）
        if state.discord_connected.load(Ordering::Relaxed) {
            if let Err(e) = update_presence_dedup(&info) {
                log::warn!("android: presence immediate update failed: {e}");
            }
        }

        if cfg!(debug_assertions) {
            crate::mlog!(
                info,
                "android: resolve: start title={} artist={} pkg={:?}",
                m(&info.title),
                m(&info.artist),
                info.package_name
            );
        }
        let url_opt = state.artwork.lock().await.resolve(&info).await;
        if cfg!(debug_assertions) {
            if let Some(ref url) = url_opt {
                // ponytail: URLはtitleに復元可能なためlenのみ
                log::info!(
                    target: crate::models::MASKED_TARGET,
                    "android: resolve: got thumbnail len={}",
                    url.len()
                );
                crate::raw_info!("android: resolve: got thumbnail url={url}");
            } else {
                crate::mlog!(
                    info,
                    "android: resolve: no thumbnail for title={}",
                    m(&info.title)
                );
            }
        }

        // ponytail: 別曲に遷移済みなら誤サムネを出さず破棄
        {
            let cur = media_state().lock().expect("media mutex poisoned").clone();
            if cur.package_name != info.package_name
                || cur.title != info.title
                || cur.artist != info.artist
            {
                crate::mlog!(
                    debug,
                    "android: resolve result discarded (stale) resolve_for={} cur={}",
                    m(&info.title),
                    m(&cur.title)
                );
                return;
            }
        }
        let Some(url) = url_opt else {
            // サムネ取得失敗時は即時送信済みなので追加送信不要
            return;
        };
        // サムネは別イベントで配信 — 情報の lastFetch/秒数に影響しない
        let payload = ThumbnailPayload {
            package_name: info.package_name.clone(),
            title: info.title.clone(),
            artist: info.artist.clone(),
            thumbnail_url: url.clone(),
        };
        let _ = app.emit("thumbnail-updated", &payload);

        if !state.discord_connected.load(Ordering::Relaxed) {
            log::debug!("android: presence update skipped (not connected)");
            return;
        }
        // サムネ付きでDiscord更新（即時送信と同一キーでdedupされるため直接送信）
        let mut with_thumb = info.clone();
        with_thumb.thumbnail_url = Some(url);
        if let Err(e) = discord_update_presence(&with_thumb) {
            log::warn!("android: presence thumb update failed: {e}");
        }
    });
}

/// 通知ボタンから呼ばれる。永続化は Kotlin 側(MediaInfoService.setRpcEnabled)で済んでいるため、
/// 接続制御とイベント発行のみ行う。
#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_MediaBridge_setRpcEnabled(
    _env: jni::EnvUnowned,
    _this: jni::objects::JObject,
    enabled: jni::sys::jboolean,
) {
    let Some(app) = APP_HANDLE.get().cloned() else {
        return;
    };
    if let Err(e) = set_rpc_enabled(&app, enabled) {
        log::error!("android: setRpcEnabled JNI failed: {e}");
    }
}
