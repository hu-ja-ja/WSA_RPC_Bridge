package com.wsarpcbridge.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.ServiceCompat

/**
 * フォアグラウンドサービス。プロセスを常駐させてメディア収集(NLS)を継続させ、
 * 現在のメディア情報を常駐通知として表示する。
 */
class MediaInfoService : Service() {

    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "foreground service started")
        createChannel(this)
        val notification = buildNotification(this, null)
        ServiceCompat.startForeground(
            this,
            NOTIFICATION_ID,
            notification,
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q)
                ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK
            else 0
        )
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_TOGGLE_RPC) {
            val enabled = !isRpcEnabled()
            setRpcEnabled(enabled)
            MediaBridge.setRpcEnabled(enabled)
            show(this, lastInfo ?: NowPlaying("", "", "", false))
        }
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    companion object {
        private const val TAG = "MediaInfoService"
        const val NOTIFICATION_ID = 1
        const val CHANNEL_ID = "now_playing"
        private const val PREFS_NAME = "media_notification"
        private const val KEY_ENABLED = "enabled"
        private const val KEY_RPC = "rpc_enabled"
        const val ACTION_TOGGLE_RPC = "com.wsarpcbridge.app.TOGGLE_RPC"

        @Volatile
        private var appContext: Context? = null

        @Volatile
        private var lastInfo: NowPlaying? = null

        @JvmStatic
        fun init(context: Context) {
            appContext = context.applicationContext
        }

        @JvmStatic
        fun isEnabled(): Boolean {
            val ctx = appContext ?: return false
            return ctx.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .getBoolean(KEY_ENABLED, false)
        }

        /** 設定変更時に Rust 側(JNI)から呼ばれる。on なら通知を開始し、権限も確認する。 */
        @JvmStatic
        fun setEnabled(enabled: Boolean) {
            val ctx = appContext ?: return
            ctx.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .edit()
                .putBoolean(KEY_ENABLED, enabled)
                .apply()
            if (enabled) {
                Log.i(TAG, "setEnabled(true)")
                start(ctx)
                // HyperOSでは権限ダイアログが表示されないため、システム設定画面で許可してもらう
                MainActivity.current?.openNotificationSettingsIfNeeded()
            } else {
                Log.i(TAG, "setEnabled(false)")
                stop(ctx)
            }
        }

        @JvmStatic
        fun isRpcEnabled(): Boolean {
            val ctx = appContext ?: return false
            return ctx.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .getBoolean(KEY_RPC, false)
        }

        @JvmStatic
        fun setRpcEnabled(enabled: Boolean) {
            val ctx = appContext ?: return
            ctx.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
                .edit()
                .putBoolean(KEY_RPC, enabled)
                .apply()
        }

        fun start(context: Context) {
            val intent = Intent(context, MediaInfoService::class.java)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, MediaInfoService::class.java))
        }

        fun createChannel(context: Context) {
            if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) return
            val manager = context.getSystemService(NotificationManager::class.java)
            val channel = NotificationChannel(
                CHANNEL_ID,
                context.getString(R.string.notif_channel_name),
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = context.getString(R.string.notif_channel_description)
                setShowBadge(false)
            }
            manager.createNotificationChannel(channel)
        }

        /** 常駐通知を更新する。サービスが起動していなくても通知は差し替わる。 */
        fun show(context: Context, info: NowPlaying) {
            lastInfo = info
            val manager = context.getSystemService(NotificationManager::class.java)
            manager.notify(NOTIFICATION_ID, buildNotification(context, info))
        }
    }
}

data class NowPlaying(
    val title: String,
    val artist: String,
    val album: String,
    val isPlaying: Boolean,
)

private fun buildNotification(context: Context, info: NowPlaying?): Notification {
    val contentIntent = PendingIntent.getActivity(
        context,
        0,
        Intent(context, MainActivity::class.java),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
    )
    val builder = NotificationCompat.Builder(context, MediaInfoService.CHANNEL_ID)
        .setSmallIcon(R.mipmap.ic_launcher)
        .setContentTitle(
            if (info != null && info.title.isNotEmpty()) info.title
            else context.getString(R.string.notif_waiting_title)
        )
        .setContentText(
            if (info != null && info.title.isNotEmpty()) {
                val parts = mutableListOf(info.artist, info.album)
                parts.add(
                    if (info.isPlaying) context.getString(R.string.notif_playing)
                    else context.getString(R.string.notif_paused)
                )
                parts.filter { it.isNotEmpty() }.joinToString(" - ")
            } else {
                context.getString(R.string.notif_waiting_text)
            }
        )
        .setOngoing(true)
        .setShowWhen(false)
        .setOnlyAlertOnce(true)
        .setContentIntent(contentIntent)

    if (info != null) {
        builder.setCategory(NotificationCompat.CATEGORY_TRANSPORT)
    }

    val actionLabel = if (MediaInfoService.isRpcEnabled()) {
        context.getString(R.string.notif_rpc_off)
    } else {
        context.getString(R.string.notif_rpc_on)
    }
    val toggleIntent = PendingIntent.getService(
        context,
        1,
        Intent(context, MediaInfoService::class.java)
            .setAction(MediaInfoService.ACTION_TOGGLE_RPC),
        PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
    )
    builder.addAction(
        NotificationCompat.Action.Builder(R.mipmap.ic_launcher, actionLabel, toggleIntent).build()
    )

    return builder.build()
}
