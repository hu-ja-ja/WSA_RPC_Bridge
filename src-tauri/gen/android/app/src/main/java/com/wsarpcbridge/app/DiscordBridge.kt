package com.wsarpcbridge.app

import android.app.Activity
import android.util.Log
import com.discord.socialsdk.DiscordSocialSdkInit

object DiscordBridge {
  private const val TAG = "DiscordBridge"

  fun init(activity: Activity) {
    try {
      // Android の C API はネイティブ側で Discord アプリの RPC サービスへ
      // bindService するため、アプリ Context の登録が必要。
      // static イニシャライザで libdiscord_partner_sdk.so もロードされる。
      DiscordSocialSdkInit.setEngineActivity(activity)
      Log.i(TAG, "SDK initialized (engine activity set)")
    } catch (e: Exception) {
      Log.e(TAG, "SDK init failed", e)
    }
  }
}
