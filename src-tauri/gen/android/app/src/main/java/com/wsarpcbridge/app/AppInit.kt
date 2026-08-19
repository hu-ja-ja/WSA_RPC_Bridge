package com.wsarpcbridge.app

import android.content.Context

/**
 * サービスだけでプロセスが再生成された場合(START_STICKY)にも、
 * MainActivity.onCreate 相当のブリッジ初期化を行う。
 * 各 init は Rust 側 OnceLock と組み合わせて冪等なので、重複実行しても安全。
 * JNI の FindClass がアプリのクラスローダーを参照するため、メインスレッドから呼ぶこと
 * (Service.onCreate / onListenerConnected はメインスレッドで実行される)。
 */
object AppInit {
    fun ensureInitialized(context: Context) {
        val ctx = context.applicationContext
        MediaWhitelistStore.init(ctx)
        MediaInfoService.init(ctx)
        NotificationBridge.initContext(ctx)
        NotificationBridge.init()
        SignatureBridge.initContext(ctx)
        SignatureBridge.init()
        MediaBridge.init()
    }
}