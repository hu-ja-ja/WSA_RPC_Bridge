package com.wsarpcbridge.app

object MediaBridge {
    init {
        System.loadLibrary("app_lib")
    }

    external fun updateMediaInfo(
        title: String,
        artist: String,
        album: String,
        packageName: String,
        positionMs: Long,
        durationMs: Long,
        isPlaying: Boolean,
    )
}
