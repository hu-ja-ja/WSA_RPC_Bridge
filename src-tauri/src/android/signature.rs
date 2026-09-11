use std::sync::OnceLock;

use jni::jni_sig;
use jni::jni_str;

static JVM: OnceLock<jni::JavaVM> = OnceLock::new();

static BRIDGE_CLASS: OnceLock<jni::objects::Global<jni::objects::JClass<'static>>> = OnceLock::new();

#[no_mangle]
pub extern "system" fn Java_com_wsarpcbridge_app_SignatureBridge_init(
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
            if let Ok(class) = env.find_class(jni_str!("com/wsarpcbridge/app/SignatureBridge")) {
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
        .ok_or("JVM not initialized (SignatureBridge.init not called)")?;
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
    BRIDGE_CLASS.get().ok_or_else(|| {
        "SignatureBridge class not cached (SignatureBridge.init not called)".to_string()
    })
}

/// 自身の署名証明書の SHA-256 フィンガープリント（コロン区切り大文字hex）。無ければ空文字。
pub fn get_signing_fingerprint() -> Result<String, String> {
    let class = bridge_class()?;
    with_jni(|env| {
        let result = env.call_static_method(
            class,
            jni_str!("getSigningFingerprint"),
            jni_sig!("()Ljava/lang/String;"),
            &[],
        )?;
        let jstr: jni::objects::JString = env.cast_local::<jni::objects::JString>(result.l()?)?;
        jstr.try_to_string(env)
    })
}
