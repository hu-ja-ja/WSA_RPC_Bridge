package com.wsarpcbridge.app

import android.content.Context
import android.content.pm.PackageManager
import android.os.Build
import android.util.Log
import java.security.MessageDigest

/**
 * 自身の署名証明書の SHA-256 フィンガープリントを返す。
 * アプリContextは MainActivity で initContext される。Rust側(JNI)から静的メソッドで呼ばれる。
 */
object SignatureBridge {
    private const val TAG = "SignatureBridge"

    init {
        System.loadLibrary("app_lib")
    }

    @Volatile
    private var appContext: Context? = null

    /** Rust側に JavaVM を渡すため、起動時に一度だけ呼ぶ。 */
    external fun init()

    @JvmStatic
    fun initContext(context: Context) {
        appContext = context.applicationContext
    }

    /** 署名証明書の SHA-256 フィンガープリント（コロン区切り大文字hex）。取得できなければ空文字。 */
    @JvmStatic
    fun getSigningFingerprint(): String {
        val ctx = appContext ?: return ""
        return try {
            val cert = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
                val info = ctx.packageManager.getPackageInfo(
                    ctx.packageName,
                    PackageManager.GET_SIGNING_CERTIFICATES
                )
                info.signingInfo?.apkContentsSigners?.firstOrNull() ?: return ""
            } else {
                @Suppress("DEPRECATION")
                ctx.packageManager.getPackageInfo(ctx.packageName, PackageManager.GET_SIGNATURES)
                    .signatures?.firstOrNull() ?: return ""
            }
            val sha256 = MessageDigest.getInstance("SHA-256").digest(cert.toByteArray())
            sha256.joinToString(":") { "%02X".format(it) }
        } catch (e: Exception) {
            Log.e(TAG, "getSigningFingerprint failed", e)
            ""
        }
    }
}
