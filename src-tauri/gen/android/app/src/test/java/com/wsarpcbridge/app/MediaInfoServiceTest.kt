package com.wsarpcbridge.app

import android.app.Application
import android.app.Notification
import android.app.NotificationManager
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.shadows.ShadowNotificationManager

@RunWith(RobolectricTestRunner::class)
class MediaInfoServiceTest {

  private lateinit var app: Application
  private lateinit var notifier: ShadowNotificationManager

  @Before
  fun setUp() {
    app = RuntimeEnvironment.getApplication()
    MediaInfoService.init(app)
    notifier = shadowOf(app.getSystemService(NotificationManager::class.java))
  }

  @Test
  fun isEnabledDefaultsToFalse() {
    assertFalse(MediaInfoService.isEnabled())
  }

  @Test
  fun setEnabledPersists() {
    MediaInfoService.setEnabled(true)
    assertTrue(MediaInfoService.isEnabled())
  }

  @Test
  fun showBuildsWaitingNotificationWhenNoMedia() {
    MediaInfoService.show(app, NowPlaying("", "", "", false))
    val notification = notifier.getNotification(MediaInfoService.NOTIFICATION_ID)!!
    assertEquals("WSA RPC Bridge", notification.extras.getString(Notification.EXTRA_TITLE))
    assertEquals("Waiting for media info...", notification.extras.getString(Notification.EXTRA_TEXT))
  }

  @Test
  fun showBuildsPlayingNotificationWithMedia() {
    MediaInfoService.show(app, NowPlaying("Song", "Artist", "Album", true))
    val notification = notifier.getNotification(MediaInfoService.NOTIFICATION_ID)!!
    assertEquals("Song", notification.extras.getString(Notification.EXTRA_TITLE))
    assertEquals("Artist - Album - Playing", notification.extras.getString(Notification.EXTRA_TEXT))
  }

  @Test
  fun showBuildsPausedNotificationWithoutAlbum() {
    MediaInfoService.show(app, NowPlaying("Song", "", "", false))
    val notification = notifier.getNotification(MediaInfoService.NOTIFICATION_ID)!!
    assertEquals("Song", notification.extras.getString(Notification.EXTRA_TITLE))
    assertEquals("Paused", notification.extras.getString(Notification.EXTRA_TEXT))
  }
}