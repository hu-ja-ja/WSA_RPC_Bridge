use std::sync::OnceLock;

use tauri::Emitter;

use crate::android::media::app_handle;

static JVM: OnceLock<jni::JavaVM> = OnceLock::new();

static BRIDGE_CLASS: OnceLock<jni::objects::GlobalRef> = OnceLock::new();

#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_NotificationBridge_init(
    mut env: jni::JNIEnv,
    _this: jni::objects::JObject,
) {
    if let Ok(vm) = env.get_java_vm() {
        let _ = JVM.set(vm);
    }
    // JNI の FindClass はメインスレッド以外ではアプリのクラスローダーを参照しないため、
    // メインスレッドでグローバル参照としてクラスを取得し、以降はそれを使う。
    if let Ok(class) = env.find_class("com/wsarpcbridge/app/NotificationBridge") {
        if let Ok(gref) = env.new_global_ref(class) {
            let _ = BRIDGE_CLASS.set(gref);
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_NotificationBridge_notifyAccessChanged(
    _env: jni::JNIEnv,
    _this: jni::objects::JObject,
    granted: jni::sys::jboolean,
) {
    if let Some(app) = app_handle() {
        let _ = app.emit("notification-access-changed", granted != 0);
    }
}

fn with_jni<T>(f: impl FnOnce(&mut jni::JNIEnv) -> jni::errors::Result<T>) -> Result<T, String> {
    let vm = JVM.get().ok_or("JVM not initialized (NotificationBridge.init not called)")?;
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    match f(&mut env) {
        Ok(v) => Ok(v),
        Err(e) => {
            let _ = env.exception_clear();
            Err(e.to_string())
        }
    }
}

fn bridge_class() -> Result<&'static jni::objects::GlobalRef, String> {
    BRIDGE_CLASS
        .get()
        .ok_or_else(|| "NotificationBridge class not cached (NotificationBridge.init not called)".to_string())
}

pub fn get_notification_access_status() -> Result<bool, String> {
    let class = bridge_class()?;
    with_jni(|env| {
        let result = env.call_static_method(class, "isAccessGranted", "()Z", &[])?;
        result.z()
    })
}

pub fn open_notification_access_settings() -> Result<(), String> {
    let class = bridge_class()?;
    with_jni(|env| {
        env.call_static_method(class, "openAccessSettings", "()V", &[])?;
        Ok(())
    })
}