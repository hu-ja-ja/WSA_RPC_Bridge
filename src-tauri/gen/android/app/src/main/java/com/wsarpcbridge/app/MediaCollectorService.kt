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

    override fun onListenerConnected() {
        super.onListenerConnected()
        startPolling()
    }

    override fun onListenerDisconnected() {
        super.onListenerDisconnected()
        MediaInfoService.show(this, NowPlaying("", "", "", false))
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
        val controller = controllers.firstOrNull { c ->
            runCatching {
                !c.metadata?.getString(MediaMetadata.METADATA_KEY_TITLE).isNullOrEmpty()
            }.getOrDefault(false)
        }

        if (controller == null) {
            MediaBridge.updateMediaInfo("", "", "", "", "", 0L, 0L, false)
            MediaInfoService.show(this, NowPlaying("", "", "", false))
            return
        }

        val metadata = controller.metadata
        val state = controller.playbackState
        val displayName = runCatching {
            val pm = packageManager
            pm.getApplicationLabel(pm.getApplicationInfo(controller.packageName, 0)).toString()
        }.getOrDefault(controller.packageName)
        val title = metadata?.getString(MediaMetadata.METADATA_KEY_TITLE).orEmpty()
        val artist = metadata?.getString(MediaMetadata.METADATA_KEY_ARTIST).orEmpty()
        val album = metadata?.getString(MediaMetadata.METADATA_KEY_ALBUM).orEmpty()
        val positionMs = state?.position ?: 0L
        val durationMs = metadata?.getLong(MediaMetadata.METADATA_KEY_DURATION)?.coerceAtLeast(0L) ?: 0L
        val isPlaying = state?.state == PlaybackState.STATE_PLAYING
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
        MediaInfoService.show(this, NowPlaying(title, artist, album, isPlaying))
    }

    companion object {
        private const val POLL_INTERVAL_MS = 2000L

        fun isNotificationAccessGranted(context: Context): Boolean {
            val nm = context.getSystemService(Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
            return nm.isNotificationListenerAccessGranted(
                ComponentName(context, MediaCollectorService::class.java)
            )
        }
    }
}
