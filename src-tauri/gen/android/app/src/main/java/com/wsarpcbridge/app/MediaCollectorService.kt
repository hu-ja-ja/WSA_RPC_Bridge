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

    override fun onListenerConnected() {
        super.onListenerConnected()
        startPolling()
    }

    override fun onListenerDisconnected() {
        super.onListenerDisconnected()
        if (MediaInfoService.isEnabled()) {
            MediaInfoService.show(this, NowPlaying("", "", "", false))
        }
    }

    override fun onNotificationPosted(sbn: StatusBarNotification?) {
        pushNow()
    }

    override fun onNotificationRemoved(sbn: StatusBarNotification?) {
        pushNow()
    }

    override fun onDestroy() {
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

        // ponytail: 無変化なら何もしない（通知の再投稿とJNI更新でCPUを起こさない）
        val now = NowPlaying(title, artist, album, isPlaying)
        if (now == lastNow) return
        lastNow = now

        if (controller == null) {
            MediaBridge.updateMediaInfo("", "", "", "", "", 0L, 0L, false)
            if (MediaInfoService.isEnabled()) {
                MediaInfoService.show(this, NowPlaying("", "", "", false))
            }
            return
        }

        val positionMs = state?.position ?: 0L
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
        if (MediaInfoService.isEnabled()) {
            MediaInfoService.show(this, now)
        }
    }

    companion object {
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
