package com.wsarpcbridge.app

import android.content.Intent
import android.content.pm.ActivityInfo
import android.content.pm.ResolveInfo
import org.junit.Assert.assertArrayEquals
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment
import org.robolectric.Shadows.shadowOf
import org.robolectric.shadows.ShadowPackageManager

@RunWith(RobolectricTestRunner::class)
class MediaWhitelistStoreTest {

  private lateinit var pm: ShadowPackageManager

  @Before
  fun setUp() {
    pm = shadowOf(RuntimeEnvironment.getApplication().packageManager)
  }

  @Test
  fun loadReturnsEmptyBeforeInit() {
    assertArrayEquals(emptyArray<String>(), MediaWhitelistStore.load())
  }

  @Test
  fun loadReturnsEmptyAfterInit() {
    MediaWhitelistStore.init(RuntimeEnvironment.getApplication())
    assertArrayEquals(emptyArray<String>(), MediaWhitelistStore.load())
  }

  @Test
  fun saveLoadRoundTrip() {
    MediaWhitelistStore.init(RuntimeEnvironment.getApplication())
    val packages = arrayOf("com.example.one", "com.example.two")
    MediaWhitelistStore.save(packages)
    assertArrayEquals(
      packages.sorted().toTypedArray(),
      MediaWhitelistStore.load().sorted().toTypedArray()
    )
  }

  @Test
  fun saveOverwritesPreviousEntries() {
    MediaWhitelistStore.init(RuntimeEnvironment.getApplication())
    MediaWhitelistStore.save(arrayOf("com.example.one"))
    MediaWhitelistStore.save(arrayOf("com.example.two"))
    assertArrayEquals(arrayOf("com.example.two"), MediaWhitelistStore.load())
  }

  @Test
  fun listAppsFormatsAndSortsByLabel() {
    MediaWhitelistStore.init(RuntimeEnvironment.getApplication())
    val intent = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
    pm.addResolveInfoForIntent(intent, resolveInfo("com.zeta.app", "Zeta Player"))
    pm.addResolveInfoForIntent(intent, resolveInfo("com.alpha.app", "Alpha Music"))

    // テストアプリ自身 (ランチャー activity を持つ) を除いて検証する
    val ownPackage = RuntimeEnvironment.getApplication().packageName
    val apps = MediaWhitelistStore.listApps()
      .filterNot { it.endsWith("\t$ownPackage") }
    assertArrayEquals(
      arrayOf("Alpha Music\tcom.alpha.app", "Zeta Player\tcom.zeta.app"),
      apps.toTypedArray()
    )
  }

  private fun resolveInfo(pkg: String, label: String): ResolveInfo {
    val labelRes = 0x7f000000 + pkg.hashCode().mod(0x10000)
    pm.addStringResource(pkg, labelRes, label)
    return ResolveInfo().apply {
      activityInfo = ActivityInfo().apply {
        packageName = pkg
        name = "$pkg.MainActivity"
        this.labelRes = labelRes
      }
    }
  }
}