package com.wsarpcbridge.app

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.util.Log
import androidx.core.content.FileProvider
import java.io.File

/**
 * アプリ内更新のための導入支援。
 * APKはRust側がキャッシュ領域に保存し、FileProvider経由で導入画面を開く。
 * アプリContextは MainActivity で initContext される。Rust側(JNI)から静的メソッドで呼ばれる。
 * 不明なアプリの許可トグルは、自動更新フローを使わない限り要求されない(オンデマンド)。
 */
object UpdateBridge {
    private const val TAG = "UpdateBridge"

    init {
        System.loadLibrary("app_lib")
    }

    @Volatile
    private var appContext: Context? = null

    /** Rust側に JavaVM を渡すため、起動時に一度だけ呼ぶ。 */
    external fun init(context: Context)

    @JvmStatic
    fun initContext(context: Context) {
        appContext = context.applicationContext
    }

    /** APK保存先。file_paths.xml の cache-path 配下のため FileProvider で共有できる。 */
    @JvmStatic
    fun cacheDirPath(): String {
        return appContext?.cacheDir?.absolutePath ?: ""
    }

    /** 不明なアプリのインストールが許可済みか。API 26 未満に個別トグルはなく true 扱い。 */
    @JvmStatic
    fun canRequestInstalls(): Boolean {
        val ctx = appContext ?: return false
        return Build.VERSION.SDK_INT < Build.VERSION_CODES.O ||
            ctx.packageManager.canRequestPackageInstalls()
    }

    /** 自アプリの「不明なアプリのインストール」設定画面を開く。 */
    @JvmStatic
    fun openInstallSettings() {
        val ctx = appContext ?: return
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            try {
                ctx.startActivity(
                    Intent(
                        Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                        Uri.parse("package:${ctx.packageName}")
                    ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                )
                return
            } catch (e: Exception) {
                Log.w(TAG, "openInstallSettings failed, falling back to app info", e)
            }
        }
        try {
            ctx.startActivity(
                Intent(
                    Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                    Uri.parse("package:${ctx.packageName}")
                ).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
            )
        } catch (e: Exception) {
            Log.w(TAG, "openAppSettings failed", e)
        }
    }

    /** 指定APKの導入画面を開く。OSの確認ダイアログは必ず表示される(サイレント導入は不可)。 */
    @JvmStatic
    fun installApk(path: String): Boolean {
        val ctx = appContext ?: return false
        return try {
            val file = File(path)
            if (!file.isFile) return false
            val uri: Uri = FileProvider.getUriForFile(
                ctx,
                "${ctx.packageName}.fileprovider",
                file
            )
            ctx.startActivity(
                Intent(Intent.ACTION_VIEW).apply {
                    setDataAndType(uri, "application/vnd.android.package-archive")
                    addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
                }
            )
            true
        } catch (e: Exception) {
            Log.e(TAG, "installApk failed", e)
            false
        }
    }
}
