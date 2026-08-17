package com.wsarpcbridge.app

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.util.Log
import androidx.activity.enableEdgeToEdge
import androidx.core.content.ContextCompat

class MainActivity : TauriActivity() {
  private val tag = "MainActivity"

  // null = 起動後まだ未確認。差分検知(未許可→許可)でのみFGSを再起動する。
  private var lastKnownPostPermission: Boolean? = null

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)

    MediaWhitelistStore.init(this)
    MediaBridge.init()

    NotificationBridge.initContext(this)
    NotificationBridge.init()
    MediaInfoService.init(this)

    SignatureBridge.initContext(this)
    SignatureBridge.init()

    DiscordBridge.init(this)
    MediaInfoService.createChannel(this)

    if (MediaInfoService.isEnabled()) {
      MediaInfoService.start(this)
    }
  }

  override fun onResume() {
    super.onResume()
    current = this
    NotificationBridge.notifyAccessChanged(
      MediaCollectorService.isNotificationAccessGranted(this)
    )
    // HyperOS等では権限ダイアログが表示されず、システム設定画面でのみ
    // POST_NOTIFICATIONS を許可できる。許可が後から付与された場合は、
    // 非表示のままだったFGS通知を再投稿するためサービスを再起動する。
    val granted = hasNotificationPermission()
    val wasGranted = lastKnownPostPermission
    lastKnownPostPermission = granted
    if (MediaInfoService.isEnabled() && granted && wasGranted == false) {
      Log.i(tag, "POST_NOTIFICATIONS granted, restarting FGS to re-post notification")
      MediaInfoService.stop(this)
      MediaInfoService.start(this)
    }
  }

  override fun onPause() {
    super.onPause()
    current = null
  }

  /** POST_NOTIFICATIONS 未許可ならシステムのアプリ通知設定画面を開く。 */
  fun openNotificationSettingsIfNeeded() {
    if (!hasNotificationPermission()) {
      Log.i(tag, "opening app notification settings")
      startActivity(
        Intent(Settings.ACTION_APP_NOTIFICATION_SETTINGS)
          .putExtra(Settings.EXTRA_APP_PACKAGE, packageName)
          .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
      )
    }
  }

  private fun hasNotificationPermission(): Boolean {
    return Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
      (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) ==
        PackageManager.PERMISSION_GRANTED)
  }

  companion object {
    @Volatile
    var current: MainActivity? = null
  }
}