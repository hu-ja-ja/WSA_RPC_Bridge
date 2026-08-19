package com.wsarpcbridge.app

import android.app.Activity
import android.util.Log

object DiscordBridge {
  private const val TAG = "DiscordBridge"

  fun init(activity: Activity) {
    try {
      // Android の C API はネイティブ側で Discord アプリの RPC サービスへ
      // bindService するため、アプリ Context の登録が必要。
      // static イニシャライザで libdiscord_partner_sdk.so もロードされる。
      // ponytail: リフレクションで呼び出すのは aar を CI(ユニットテスト)へ
      // 同梱しないため。実行時は aar が同梱されるので挙動は不変。
      val initClass = Class.forName("com.discord.socialsdk.DiscordSocialSdkInit")
      initClass.getMethod("setEngineActivity", Activity::class.java)
        .invoke(null, activity)
      Log.i(TAG, "SDK initialized (engine activity set)")
    } catch (e: Exception) {
      Log.e(TAG, "SDK init failed", e)
    }
  }
}
