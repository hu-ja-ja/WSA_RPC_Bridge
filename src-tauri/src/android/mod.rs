use std::ffi::{c_char, c_void, CString};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use crate::commands::AppState;
use crate::models::MediaInfo;

const APP_ID: u64 = 1530562506513449120;

static MEDIA_STATE: OnceLock<Mutex<MediaInfo>> = OnceLock::new();

/// 無再生が1時間続いて切断した後、ユーザーがRPCをOFF/ONするか再起動するまで
/// 再接続を抑止するフラグ。
static RPC_IDLE: AtomicBool = AtomicBool::new(false);

static APP_HANDLE: OnceLock<AppHandle> = OnceLock::new();
static LAST_PRESENCE_KEY: Mutex<String> = Mutex::new(String::new());

pub fn set_app_handle(app: AppHandle) {
    let _ = APP_HANDLE.set(app);
}

pub fn rpc_idle() -> bool {
    RPC_IDLE.load(Ordering::SeqCst)
}

// ---- cdiscord.h FFI 宣言 (Discord Social SDK 1.10) ----

#[repr(C)]
#[derive(Clone, Copy)]
struct Discord_String {
    ptr: *mut u8,
    size: usize,
}

struct Discord_Client {
    opaque: *mut c_void,
}

struct Discord_Activity {
    opaque: *mut c_void,
}

struct Discord_ActivityTimestamps {
    opaque: *mut c_void,
}

struct Discord_ActivityAssets {
    opaque: *mut c_void,
}

struct Discord_ClientResult {
    opaque: *mut c_void,
}

unsafe impl Send for Discord_Client {}
unsafe impl Sync for Discord_Client {}

type Cb = unsafe extern "C" fn(*mut Discord_ClientResult, *mut c_void);

#[derive(Clone, Copy)]
struct Sdk {
    client_init: unsafe extern "C" fn(*mut Discord_Client),
    client_set_application_id: unsafe extern "C" fn(*mut Discord_Client, u64),
    client_update_rich_presence:
        unsafe extern "C" fn(*mut Discord_Client, *mut Discord_Activity, Cb, *mut c_void, *mut c_void),
    client_drop: unsafe extern "C" fn(*mut Discord_Client),
    activity_init: unsafe extern "C" fn(*mut Discord_Activity),
    activity_set_name: unsafe extern "C" fn(*mut Discord_Activity, Discord_String),
    activity_set_type: unsafe extern "C" fn(*mut Discord_Activity, i32),
    activity_set_state: unsafe extern "C" fn(*mut Discord_Activity, *mut Discord_String),
    activity_set_details: unsafe extern "C" fn(*mut Discord_Activity, *mut Discord_String),
    activity_set_timestamps: unsafe extern "C" fn(*mut Discord_Activity, *mut Discord_ActivityTimestamps),
    activity_set_assets: unsafe extern "C" fn(*mut Discord_Activity, *mut Discord_ActivityAssets),
    activity_drop: unsafe extern "C" fn(*mut Discord_Activity),
    ts_init: unsafe extern "C" fn(*mut Discord_ActivityTimestamps),
    ts_set_start: unsafe extern "C" fn(*mut Discord_ActivityTimestamps, u64),
    ts_set_end: unsafe extern "C" fn(*mut Discord_ActivityTimestamps, u64),
    ts_drop: unsafe extern "C" fn(*mut Discord_ActivityTimestamps),
    assets_init: unsafe extern "C" fn(*mut Discord_ActivityAssets),
    assets_set_large_image: unsafe extern "C" fn(*mut Discord_ActivityAssets, *mut Discord_String),
    assets_set_large_text: unsafe extern "C" fn(*mut Discord_ActivityAssets, *mut Discord_String),
    assets_drop: unsafe extern "C" fn(*mut Discord_ActivityAssets),
    run_callbacks: unsafe extern "C" fn(),
    set_free_threaded: unsafe extern "C" fn(),
}

static SDK: OnceLock<Sdk> = OnceLock::new();
static CLIENT: OnceLock<Mutex<Option<Discord_Client>>> = OnceLock::new();
static CALLBACKS: OnceLock<Mutex<Option<JoinHandle<()>>>> = OnceLock::new();
static CALLBACKS_RUNNING: AtomicBool = AtomicBool::new(false);

unsafe fn load_symbol<T: Copy>(handle: *mut c_void, name: &str) -> Result<T, String> {
    let sym = CString::new(name).unwrap();
    let ptr = libc::dlsym(handle, sym.as_ptr());
    if ptr.is_null() {
        return Err(format!("dlsym failed for {name}"));
    }
    Ok(std::mem::transmute_copy(&ptr))
}

fn load_sdk() -> Result<&'static Sdk, String> {
    if let Some(sdk) = SDK.get() {
        return Ok(sdk);
    }
    let sdk = unsafe {
        let name = CString::new("libdiscord_partner_sdk.so").unwrap();
        let handle = libc::dlopen(name.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL);
        if handle.is_null() {
            return Err(format!("dlopen failed for libdiscord_partner_sdk.so"));
        }
        Sdk {
            client_init: load_symbol(handle, "Discord_Client_Init")?,
            client_set_application_id: load_symbol(handle, "Discord_Client_SetApplicationId")?,
            client_update_rich_presence: load_symbol(handle, "Discord_Client_UpdateRichPresence")?,
            client_drop: load_symbol(handle, "Discord_Client_Drop")?,
            activity_init: load_symbol(handle, "Discord_Activity_Init")?,
            activity_set_name: load_symbol(handle, "Discord_Activity_SetName")?,
            activity_set_type: load_symbol(handle, "Discord_Activity_SetType")?,
            activity_set_state: load_symbol(handle, "Discord_Activity_SetState")?,
            activity_set_details: load_symbol(handle, "Discord_Activity_SetDetails")?,
            activity_set_timestamps: load_symbol(handle, "Discord_Activity_SetTimestamps")?,
            activity_set_assets: load_symbol(handle, "Discord_Activity_SetAssets")?,
            activity_drop: load_symbol(handle, "Discord_Activity_Drop")?,
            ts_init: load_symbol(handle, "Discord_ActivityTimestamps_Init")?,
            ts_set_start: load_symbol(handle, "Discord_ActivityTimestamps_SetStart")?,
            ts_set_end: load_symbol(handle, "Discord_ActivityTimestamps_SetEnd")?,
            ts_drop: load_symbol(handle, "Discord_ActivityTimestamps_Drop")?,
            assets_init: load_symbol(handle, "Discord_ActivityAssets_Init")?,
            assets_set_large_image: load_symbol(handle, "Discord_ActivityAssets_SetLargeImage")?,
            assets_set_large_text: load_symbol(handle, "Discord_ActivityAssets_SetLargeText")?,
            assets_drop: load_symbol(handle, "Discord_ActivityAssets_Drop")?,
            run_callbacks: load_symbol(handle, "Discord_RunCallbacks")?,
            set_free_threaded: load_symbol(handle, "Discord_SetFreeThreaded")?,
        }
    };
    SDK.set(sdk).map_err(|_| "discord sdk already initialized".to_string())?;
    Ok(SDK.get().unwrap())
}

fn client() -> Result<std::sync::MutexGuard<'static, Option<Discord_Client>>, String> {
    CLIENT
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "discord client mutex poisoned".to_string())
}

fn to_discord_string(s: &str) -> Result<Discord_String, String> {
    let c = CString::new(s).map_err(|e| e.to_string())?;
    let len = c.as_bytes().len();
    Ok(Discord_String {
        ptr: c.into_raw() as *mut u8,
        size: len,
    })
}

fn free_discord_string(s: &mut Discord_String) {
    if !s.ptr.is_null() {
        unsafe {
            drop(CString::from_raw(s.ptr as *mut c_char));
        }
        s.ptr = std::ptr::null_mut();
        s.size = 0;
    }
}

// 非認証RPC: SetApplicationId 済みクライアントで UpdateRichPresence するだけで
// Discord アプリの RPC サービスへ接続される (Connect() 不要)

pub fn discord_connect() -> Result<(), String> {
    if rpc_idle() {
        return Ok(());
    }
    let sdk = load_sdk()?;
    let mut guard = client()?;
    if guard.is_some() {
        return Ok(());
    }
    unsafe {
        (sdk.set_free_threaded)();
    }
    let mut c = Discord_Client {
        opaque: std::ptr::null_mut(),
    };
    unsafe {
        (sdk.client_init)(&mut c);
        (sdk.client_set_application_id)(&mut c, APP_ID);
    }
    *guard = Some(c);
    start_callbacks();
    log::info!("discord: connected via Social SDK C API (app id {APP_ID})");
    Ok(())
}

pub fn discord_update_presence(info: &MediaInfo) -> Result<(), String> {
    if rpc_idle() {
        return Ok(());
    }
    let sdk = load_sdk()?;
    let guard = client()?;
    let Some(c) = guard.as_ref() else {
        return Err("discord: client not connected".to_string());
    };
    let c_ptr = c as *const Discord_Client as *mut Discord_Client;

    // デスクトップ実装 (discord/rpc.rs) と同じマッピング:
    // name=title, details=display_name??package_name, state=title, type=Listening,
    // start=now-pos/1000, end=start+dur/1000 (再生中), large_image=thumbnail, large_text=artist
    let mut name = to_discord_string(&info.title)?;
    let mut details = to_discord_string(info.display_name.as_deref().unwrap_or(&info.package_name))?;
    let mut state = to_discord_string(&info.title)?;
    let mut artist_str: Option<Discord_String> = None;
    let mut thumb_str: Option<Discord_String> = None;
    let mut ts = Discord_ActivityTimestamps {
        opaque: std::ptr::null_mut(),
    };
    let mut assets = Discord_ActivityAssets {
        opaque: std::ptr::null_mut(),
    };
    let mut activity = Discord_Activity {
        opaque: std::ptr::null_mut(),
    };

    // value で渡す name は clone を渡し、元は後で free する
    let result = (|| {
        unsafe {
            (sdk.activity_init)(&mut activity);
            (sdk.activity_set_name)(&mut activity, name);
            (sdk.activity_set_type)(&mut activity, 2); // Listening
            (sdk.activity_set_state)(&mut activity, &mut state);
            (sdk.activity_set_details)(&mut activity, &mut details);

            if info.is_playing {
                if let Some(pos) = info.position.filter(|p| *p > 0) {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let start = now.saturating_sub(pos / 1000);
                    (sdk.ts_init)(&mut ts);
                    (sdk.ts_set_start)(&mut ts, start);
                    if let Some(dur) = info.duration.filter(|d| *d > 0) {
                        (sdk.ts_set_end)(&mut ts, start + dur / 1000);
                    }
                    (sdk.activity_set_timestamps)(&mut activity, &mut ts);
                }
            }

            if let Some(url) = &info.thumbnail_url {
                thumb_str = Some(to_discord_string(url)?);
                artist_str = Some(to_discord_string(&info.artist)?);
                (sdk.assets_init)(&mut assets);
                (sdk.assets_set_large_image)(&mut assets, thumb_str.as_mut().unwrap());
                (sdk.assets_set_large_text)(&mut assets, artist_str.as_mut().unwrap());
                (sdk.activity_set_assets)(&mut activity, &mut assets);
            }

            (sdk.client_update_rich_presence)(c_ptr, &mut activity, update_presence_cb, std::ptr::null_mut(), std::ptr::null_mut());
            log::debug!("discord: presence update sent: {} - {}", info.title, info.artist);
        }
        Ok::<(), String>(())
    })();

    // cleanup
    unsafe {
        if !activity.opaque.is_null() {
            (sdk.activity_drop)(&mut activity);
        }
        if !ts.opaque.is_null() {
            (sdk.ts_drop)(&mut ts);
        }
        if !assets.opaque.is_null() {
            (sdk.assets_drop)(&mut assets);
        }
    }
    free_discord_string(&mut name);
    free_discord_string(&mut details);
    free_discord_string(&mut state);
    if let Some(mut s) = thumb_str {
        free_discord_string(&mut s);
    }
    if let Some(mut s) = artist_str {
        free_discord_string(&mut s);
    }
    result
}

pub fn discord_disconnect() -> Result<(), String> {
    RPC_IDLE.store(false, Ordering::SeqCst);
    *LAST_PRESENCE_KEY.lock().expect("presence key mutex poisoned") = String::new();
    do_disconnect()
}

/// 無再生アイドルによる自動切断。`RPC_IDLE`を維持して再接続を抑止する。
pub fn discord_idle_disconnect() -> Result<(), String> {
    RPC_IDLE.store(true, Ordering::SeqCst);
    do_disconnect()
}

fn do_disconnect() -> Result<(), String> {
    let sdk = load_sdk()?;
    stop_callbacks();
    let mut guard = client()?;
    if let Some(mut c) = guard.take() {
        let c_ptr = &mut c as *mut Discord_Client;
        unsafe {
            (sdk.client_drop)(c_ptr);
        }
        log::info!("discord: disconnected");
    }
    Ok(())
}

unsafe extern "C" fn update_presence_cb(_result: *mut Discord_ClientResult, _user: *mut c_void) {
    log::debug!("discord: presence update callback");
}

fn start_callbacks() {
    if CALLBACKS_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }
    let handle = thread::spawn(|| loop {
        if !CALLBACKS_RUNNING.load(Ordering::SeqCst) {
            break;
        }
        if let Ok(sdk) = load_sdk() {
            unsafe {
                (sdk.run_callbacks)();
            }
        }
        thread::sleep(Duration::from_millis(250));
    });
    CALLBACKS
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map(|mut h| *h = Some(handle))
        .ok();
}

fn stop_callbacks() {
    CALLBACKS_RUNNING.store(false, Ordering::SeqCst);
    if let Ok(mut h) = CALLBACKS.get_or_init(|| Mutex::new(None)).lock() {
        if let Some(handle) = h.take() {
            let _ = handle.join();
        }
    }
}

pub fn media_state() -> &'static Mutex<MediaInfo> {
    MEDIA_STATE.get_or_init(|| Mutex::new(MediaInfo::default()))
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

        if info.title.is_empty()
            || !state
                .discord_connected
                .load(Ordering::Relaxed)
        {
            return;
        }
        let key = format!(
            "{}|{}|{}|{}",
            info.title, info.artist, info.album, info.is_playing
        );
        let mut last = LAST_PRESENCE_KEY.lock().expect("presence key mutex poisoned");
        if *last == key {
            return;
        }
        *last = key;
        drop(last);
        if let Err(e) = discord_update_presence(&info) {
            log::warn!("android: presence update failed: {e}");
        }
    });
}
