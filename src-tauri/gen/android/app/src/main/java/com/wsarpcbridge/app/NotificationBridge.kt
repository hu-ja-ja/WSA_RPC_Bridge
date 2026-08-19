package com.wsarpcbridge.app

import android.content.Context
import android.content.Intent
import android.provider.Settings

/**
 * 通知アクセス権限の確認・設定画面起動。
 * アプリContextは MainActivity で initContext される。Rust側(JNI)から静的メソッドで呼ばれる。
 */
object NotificationBridge {
    init {
        System.loadLibrary("app_lib")
    }

    @Volatile
    private var appContext: Context? = null

    /** Rust側に JavaVM を渡すため、起動時に一度だけ呼ぶ。 */
    external fun init()

    /** 権限状態の変化を Rust 経由で webview へ通知する。 */
    external fun notifyAccessChanged(granted: Boolean)

    @JvmStatic
    fun initContext(context: Context) {
        appContext = context.applicationContext
    }

    @JvmStatic
    fun isAccessGranted(): Boolean {
        val ctx = appContext ?: return false
        return MediaCollectorService.isNotificationAccessGranted(ctx)
    }

    @JvmStatic
    fun openAccessSettings() {
        val ctx = appContext ?: return
        ctx.startActivity(
            Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        )
    }
}