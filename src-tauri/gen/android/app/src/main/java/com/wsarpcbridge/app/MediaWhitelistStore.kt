package com.wsarpcbridge.app

import android.content.Context
import android.content.Intent

/**
 * メディア検出ホワイトリストの保存と、選択候補のアプリ一覧取得。
 * アプリContextは MainActivity で init される。Rust側(JNI)から静的メソッドで呼ばれる。
 */
object MediaWhitelistStore {
    private const val PREFS_NAME = "media_whitelist"
    private const val KEY_PACKAGES = "packages"

    @Volatile
    private var appContext: Context? = null

    @JvmStatic
    fun init(context: Context) {
        appContext = context.applicationContext
    }

    @JvmStatic
    fun load(): Array<String> {
        val ctx = appContext ?: return emptyArray()
        return ctx.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .getStringSet(KEY_PACKAGES, emptySet())
            .orEmpty()
            .toTypedArray()
    }

    @JvmStatic
    fun save(packages: Array<String>) {
        val ctx = appContext ?: return
        ctx.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
            .edit()
            .putStringSet(KEY_PACKAGES, packages.toSet())
            .apply()
    }

    /** ランチャーに表示される全アプリを "表示名\tパッケージ名" の配列で返す。 */
    @JvmStatic
    fun listApps(): Array<String> {
        val ctx = appContext ?: return emptyArray()
        val pm = ctx.packageManager
        val intent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
        return pm.queryIntentActivities(intent, 0)
            .distinctBy { it.activityInfo.packageName }
            .map { ri -> "${ri.loadLabel(pm)}\t${ri.activityInfo.packageName}" }
            .sortedBy { it.substringBefore('\t').lowercase() }
            .toTypedArray()
    }
}
