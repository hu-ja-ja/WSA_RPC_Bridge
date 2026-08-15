use std::sync::{Mutex, OnceLock};

use jni::objects::{JObject, JString};
use jni::sys::{jboolean, jlong};
use jni::JNIEnv;

use crate::models::MediaInfo;

static MEDIA_STATE: OnceLock<Mutex<MediaInfo>> = OnceLock::new();

pub fn media_state() -> &'static Mutex<MediaInfo> {
    MEDIA_STATE.get_or_init(|| Mutex::new(MediaInfo::default()))
}

#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_MediaBridge_updateMediaInfo(
    mut env: JNIEnv,
    _this: JObject,
    title: JString,
    artist: JString,
    album: JString,
    package_name: JString,
    position_ms: jlong,
    duration_ms: jlong,
    is_playing: jboolean,
) {
    let mut get = |s: &JString| env.get_string(s).map(|v| v.into()).unwrap_or_default();

    let info = MediaInfo {
        title: get(&title),
        artist: get(&artist),
        album: get(&album),
        package_name: get(&package_name),
        thumbnail_url: None,
        position: (position_ms > 0).then_some(position_ms as u64),
        duration: (duration_ms > 0).then_some(duration_ms as u64),
        display_name: None,
        is_playing: is_playing != 0,
    };

    log::info!("android: media update: {} - {} (playing={})", info.title, info.artist, info.is_playing);
    *media_state().lock().expect("media mutex poisoned") = info;
}
