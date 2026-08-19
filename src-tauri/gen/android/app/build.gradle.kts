import java.util.Base64
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

fun keyFrom(name: String, prop: String): String? {
    val propFile = file("keystore.properties")
    if (propFile.exists()) {
        val props = Properties().apply { propFile.inputStream().use { load(it) } }
        props.getProperty(prop)?.let { return it }
    }
    return System.getenv(name)
}

val keystoreFile = keyFrom("KEYSTORE_FILE", "storeFile")?.let { file(it) }
    ?: keyFrom("KEYSTORE_BASE64", "storeBase64")?.let { b64 ->
        val decoded = Base64.getDecoder().decode(b64)
        file("${layout.buildDirectory.get().asFile}/release.keystore")
            .apply { parentFile.mkdirs() }
            .also { it.writeBytes(decoded) }
    }

android {
    compileSdk = 36
    namespace = "com.wsarpcbridge.app"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "com.wsarpcbridge.app"
        minSdk = 24
        targetSdk = 36
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    signingConfigs {
        create("release") {
            storeFile = keystoreFile
            storePassword = keyFrom("KEYSTORE_PASSWORD", "storePassword")
            keyAlias = keyFrom("KEY_ALIAS", "keyAlias")
            keyPassword = keyFrom("KEY_PASSWORD", "keyPassword")
        }
    }
    buildTypes {
        getByName("debug") {
            applicationIdSuffix = ".debug"
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            if (keystoreFile != null) {
                signingConfig = signingConfigs.getByName("release")
            }
            isMinifyEnabled = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
        }
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        buildConfig = true
    }
    testOptions {
        unitTests.isIncludeAndroidResources = true
        // Robolectric は JDK 21 で Dynamic Agent ロードが必要
        unitTests.all { it.jvmArgs("-XX:+EnableDynamicAgentLoading") }
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    // Discord SDK はライセンス上リポジトリに置けず、ビルド時に取得される。
    // 無い環境(CI のユニットテスト)では DiscordBridge はリフレクションで呼ぶので依存を落とす。
    if (file("libs/discord_partner_sdk.aar").exists()) {
        implementation(files("libs/discord_partner_sdk.aar"))
    }
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.robolectric:robolectric:4.16.1")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")