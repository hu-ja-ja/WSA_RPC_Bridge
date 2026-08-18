package com.wsarpcbridge.app

import android.content.ComponentName
import android.content.Context
import android.media.MediaMetadata
import android.media.session.MediaSessionManager
import android.media.session.PlaybackState
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import java.util.Timer
import java.util.TimerTask

class MediaCollectorService : NotificationListenerService() {

    private var timer: Timer? = null
    private var lastNow: NowPlaying? = null
    private var lastPositionMs: Long = -1L

    override fun onListenerConnected() {
        super.onListenerConnected()
        isConnected = true
        startPolling()
    }

    override fun onListenerDisconnected() {
        super.onListenerDisconnected()
        isConnected = false
        if (MediaInfoService.isEnabled()) {
            MediaInfoService.show(this, NowPlaying("", "", "", false))
        }
    }

    override fun onNotificationPosted(sbn: StatusBarNotification?) {
        // 自分のアプリの通知(常駐通知)は無視する。無視しないと
        // show() -> notify() -> onNotificationPosted -> pushNow() -> show() ... の
        // 自己増幅ループで main スレッドと system_server を常時暴走させる。
        if (sbn?.packageName == packageName) return
        pushNow()
    }

    override fun onNotificationRemoved(sbn: StatusBarNotification?) {
        if (sbn?.packageName == packageName) return
        pushNow()
    }

    override fun onDestroy() {
        isConnected = false
        timer?.cancel()
        super.onDestroy()
    }

    private fun startPolling() {
        timer?.cancel()
        timer = Timer().apply {
            schedule(object : TimerTask() {
                override fun run() {
                    try {
                        pushNow()
                    } catch (_: Exception) {
                    }
                }
            }, 0, POLL_INTERVAL_MS)
        }
    }

    private fun pushNow() {
        val sessionManager = getSystemService(Context.MEDIA_SESSION_SERVICE) as MediaSessionManager
        val controllers = sessionManager.getActiveSessions(ComponentName(this, MediaCollectorService::class.java))
        // ホワイトリストに含まれるアプリのセッションのみ検出する。空なら何も検出しない。
        val whitelist = MediaWhitelistStore.load().toSet()
        val controller = controllers.firstOrNull { c ->
            runCatching {
                c.packageName in whitelist &&
                    !c.metadata?.getString(MediaMetadata.METADATA_KEY_TITLE).isNullOrEmpty()
            }.getOrDefault(false)
        }

        val metadata = controller?.metadata
        val state = controller?.playbackState
        val displayName = controller?.let { c ->
            runCatching {
                val pm = packageManager
                pm.getApplicationLabel(pm.getApplicationInfo(c.packageName, 0)).toString()
            }.getOrDefault(c.packageName)
        }.orEmpty()
        val title = metadata?.getString(MediaMetadata.METADATA_KEY_TITLE).orEmpty()
        val artist = metadata?.getString(MediaMetadata.METADATA_KEY_ARTIST).orEmpty()
        val album = metadata?.getString(MediaMetadata.METADATA_KEY_ALBUM).orEmpty()
        val isPlaying = state?.state == PlaybackState.STATE_PLAYING
        val positionMs = state?.position ?: 0L

        // ponytail: 通知の再投稿は曲情報・状態変化のときだけ。位置進みでは再投稿しない
        // （notify()はsystem_server/SystemUI/surfaceflingerを消費する重い処理）。
        // Discord側は位置進みでも送って秒をライブに保つ（シークも位置変化として自動反映）。
        // ループは自パッケージ通知のスキップで断たれているので頻度は外部イベントで上限される。
        val now = NowPlaying(title, artist, album, isPlaying)
        val infoChanged = now != lastNow
        val positionChanged = positionMs != lastPositionMs
        lastNow = now
        lastPositionMs = positionMs
        if (!infoChanged && !positionChanged) return

        if (controller == null) {
            if (infoChanged) {
                MediaBridge.updateMediaInfo("", "", "", "", "", 0L, 0L, false)
                if (MediaInfoService.isEnabled()) {
                    MediaInfoService.show(this, NowPlaying("", "", "", false))
                }
            }
            return
        }

        val durationMs = metadata?.getLong(MediaMetadata.METADATA_KEY_DURATION)?.coerceAtLeast(0L) ?: 0L
        MediaBridge.updateMediaInfo(
            title = title,
            artist = artist,
            album = album,
            packageName = controller.packageName,
            displayName = displayName,
            positionMs = positionMs,
            durationMs = durationMs,
            isPlaying = isPlaying,
        )
        if (infoChanged && MediaInfoService.isEnabled()) {
            MediaInfoService.show(this, now)
        }
    }

    companion object {
        /** システムにバインドされているか。false なら onResume で強制リバインド対象。 */
        @Volatile
        var isConnected: Boolean = false
            private set

        // 保険のポーリング。通常は onNotificationPosted/Removed のイベント駆動で更新される。
        private const val POLL_INTERVAL_MS = 30_000L

        fun isNotificationAccessGranted(context: Context): Boolean {
            val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
            return nm.isNotificationListenerAccessGranted(
                ComponentName(context, MediaCollectorService::class.java)
            )
        }
    }
}
