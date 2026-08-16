# Privacy Policy

[English](PRIVACY_POLICY_en.md) | [日本語](PRIVACY_POLICY.md)

Effective date: August 5, 2026
Revised date: August 17, 2026

## 1. Introduction

WSA RPC Bridge (hereafter "this app") is an application that retrieves media playback information from apps running on WSA (Windows Subsystem for Android) and displays it on Discord Rich Presence.

This app comes in two forms:

- **Windows version**: A desktop application running on Windows PC. Retrieves media playback information from WSA via ADB.
- **Android version**: An application running on Android device. Retrieves media playback information directly within the device.

This policy describes the data handled by this app and where it is sent.

Please also see the [Terms of Service](TERMS_OF_SERVICE_en.md) when using this app.

## 2. Collection of Personal Information

This app does not collect or transmit personal information to the developer. The following features are not implemented at all:

- Telemetry / usage analytics
- Advertising
- Account creation / login
- Automatic crash report submission
- External transmission of logs

## 3. Core Required Features

These features are essential for this app to operate.

### Media Playback Information Retrieval (Windows version)

This app uses ADB to retrieve media playback information from WSA. ADB communication is local only (communication with WSA on the same PC) and no data is sent to external servers.

### Media Playback Information Retrieval (Android version)

This app uses the device's notification access permission and retrieves the current media playback information directly within the device from Android's MediaSession. All retrieval is completed within the device, and no data is sent to external servers.

Only apps added to the whitelist in the settings are detected.

### Persistent Notification (Android version)

This app runs as a foreground service and displays the currently playing media information (title, artist, album, playback state) as a notification on the device. This is a display on the device only, and the notification content is not transmitted externally.

### Permissions (Android version)

This app requests the following permissions.

| Permission | Type | Purpose |
|---|---|---|
| Notification access | Special access (granted manually in settings) | Detect Android's MediaSession to retrieve media playback information |
| Display notifications (Android 13+) | Runtime permission | Display the persistent notification |
| Run foreground service | Normal permission | Keep a service running to continue collecting media playback information |
| Internet | Normal permission | Communication for sending to Discord and resolving thumbnails |

The notification access permission grants the ability to view other apps' notifications, but this app uses it only to detect media sessions, and **the ability to read other apps' notification content is not implemented**. It is never used to view, store, transmit, or otherwise handle notification content. This can also be verified in the source code.

### Discord Rich Presence (Common)

This is the main feature of this app. **Only when Discord Rich Presence is enabled in settings**, the currently playing media information is sent to Discord and displayed as a Discord activity.

- In the Windows version, it is sent via the Discord client.
- In the Android version, it is sent by connecting directly to the Discord app on the device.

Data sent:

- Media title (song title / video title)
- Artist name
- Name of the app currently playing
- Thumbnail image URL
- Playback position and duration

Depending on your Discord settings, the displayed content may be visible to other users (e.g., friends). For information on how Discord handles your data, please refer to [Discord's Privacy Policy](https://discord.com/privacy).

## 4. Optional Third-Party Integrations

This app provides integration features for specific apps through a plugin architecture (`ArtworkResolver`). **These features only operate while the corresponding app is playing, and data is sent to external servers only at that time.** This integration feature works in both the Windows and Android versions.

| Integrated app | Purpose | Data sent | Destination | When sent |
|---|---|---|---|---|
| vocacolle (`jp.nicovideo.nicobox`) | Retrieve thumbnail image | Media title (song title / video title) | Niconico Search API (snapshot.search.nicovideo.jp) | Only while playing |

Resolved thumbnails are not saved as image files; only the URL is held temporarily in memory within the app and sent to Discord Rich Presence. The image itself is not downloaded or stored by this app; it is retrieved and displayed by Discord.

### Future Additions to Integrations

New third-party integrations may be added in the future due to the plugin architecture described above. If an integration is added, this policy will be updated, and the principle that "data is only transmitted while the corresponding feature is in use" will always be maintained.

## 5. Local Data Storage and Deletion

This app stores the following data locally only.

| Platform | Data | Location | Contents |
|---|---|---|---|
| Windows | Configuration file | `%APPDATA%\wsa-rpc-bridge\config.json` | App settings (auto-start, cache settings, etc.) |
| Windows | APK cache | `%LOCALAPPDATA%\wsa-rpc-bridge\ApkCache` | Temporary files for resolving app names |
| Android | Settings and whitelist | App internal storage (SharedPreferences) | App settings, whitelist of apps to detect |

The Windows version data is not automatically deleted when the app is uninstalled. To delete it, manually remove the `wsa-rpc-bridge` folders listed above.

The Android version data is deleted from the device when the app is uninstalled.

## 6. Contact

If you have any questions regarding this policy, please contact:

- GitHub Issues: https://github.com/hu-ja-ja/WSA_RPC_Bridge/issues
- Mail: `hujaja.jp@gmail.com`

## 7. Changes to This Policy

This policy may be revised in response to feature additions/changes or changes in applicable laws. When revised, this page will be updated along with the revised date.

Changes may take effect without prior notice to you.
