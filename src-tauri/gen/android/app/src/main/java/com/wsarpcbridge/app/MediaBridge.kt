package com.wsarpcbridge.app

object MediaBridge {
    init {
        System.loadLibrary("app_lib")
    }

    /** Rust側に JavaVM を渡すため、起動時に一度だけ呼ぶ。 */
    external fun init()

    external fun updateMediaInfo(
        title: String,
        artist: String,
        album: String,
        packageName: String,
        displayName: String,
        positionMs: Long,
        durationMs: Long,
        isPlaying: Boolean,
    )

    /** 通知ボタンから RPC を ON/OFF する。永続化は MediaInfoService.setRpcEnabled 側で行う。 */
    external fun setRpcEnabled(enabled: Boolean)
}
