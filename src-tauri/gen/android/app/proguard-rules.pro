# Add project specific ProGuard rules here.
# You can control the set of applied configuration files using the
# proguardFiles setting in build.gradle.
#
# For more details, see
#   http://developer.android.com/guide/developing/tools/proguard.html

# Rust(JNI) が名前文字列で GetStaticMethodID するため、R8 のリネーム/削除から守る。
# 守らないと release ビルドで NoSuchMethodError になる。
-keep class com.wsarpcbridge.app.MediaWhitelistStore { public static *; }
-keep class com.wsarpcbridge.app.MediaInfoService { public static *; }
-keep class com.wsarpcbridge.app.NotificationBridge { public static *; }
-keep class com.wsarpcbridge.app.SignatureBridge { public static *; }
-keep class com.wsarpcbridge.app.UpdateBridge { public static *; }
-keepclasseswithmembernames class * { native <methods>; }

# 証明書検証は Rust から JNI 経由でのみ参照されるため、R8 には不要に見える。
# 守らないと release ビルドで ClassNotFoundException になり全 HTTPS 通信が失敗する。
-keep, includedescriptorclasses class org.rustls.platformverifier.** { *; }

# If your project uses WebView with JS, uncomment the following
# and specify the fully qualified class name to the JavaScript interface
# class:
#-keepclassmembers class fqcn.of.javascript.interface.for.webview {
#   public *;
#}

# Uncomment this to preserve the line number information for
# debugging stack traces.
#-keepattributes SourceFile,LineNumberTable

# If you keep the line number information, uncomment this to
# hide the original source file name.
#-renamesourcefileattribute SourceFile