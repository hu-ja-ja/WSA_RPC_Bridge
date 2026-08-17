use std::sync::OnceLock;

static JVM: OnceLock<jni::JavaVM> = OnceLock::new();

static BRIDGE_CLASS: OnceLock<jni::objects::GlobalRef> = OnceLock::new();

#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_SignatureBridge_init(
    mut env: jni::JNIEnv,
    _this: jni::objects::JObject,
) {
    if let Ok(vm) = env.get_java_vm() {
        let _ = JVM.set(vm);
    }
    // JNI の FindClass はメインスレッド以外ではアプリのクラスローダーを参照しないため、
    // メインスレッドでグローバル参照としてクラスを取得し、以降はそれを使う。
    if let Ok(class) = env.find_class("com/wsarpcbridge/app/SignatureBridge") {
        if let Ok(gref) = env.new_global_ref(class) {
            let _ = BRIDGE_CLASS.set(gref);
        }
    }
}

fn with_jni<T>(f: impl FnOnce(&mut jni::JNIEnv) -> jni::errors::Result<T>) -> Result<T, String> {
    let vm = JVM.get().ok_or("JVM not initialized (SignatureBridge.init not called)")?;
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
        .ok_or_else(|| "SignatureBridge class not cached (SignatureBridge.init not called)".to_string())
}

/// 自身の署名証明書の SHA-256 フィンガープリント（コロン区切り大文字hex）。無ければ空文字。
pub fn get_signing_fingerprint() -> Result<String, String> {
    let class = bridge_class()?;
    with_jni(|env| {
        let result =
            env.call_static_method(class, "getSigningFingerprint", "()Ljava/lang/String;", &[])?;
        let jstr = jni::objects::JString::from(result.l()?);
        env.get_string(&jstr).map(|s| s.into())
    })
}
