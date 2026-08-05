# Privacy Policy

[English](PRIVACY_POLICY_en.md) | [日本語](PRIVACY_POLICY.md)

Effective date: August 5, 2026

## 1. Introduction

WSA RPC Bridge (hereafter "this app") is a desktop application that retrieves media playback information from apps running on WSA (Windows Subsystem for Android) via ADB and displays it on Discord Rich Presence.

This policy describes the data handled by this app and where it is sent.

## 2. Collection of Personal Information

This app does not collect or transmit personal information to the developer. The following features are not implemented at all:

- Telemetry / usage analytics
- Advertising
- Account creation / login
- Automatic crash report submission
- External transmission of logs

## 3. Core Required Features

These features are essential for this app to operate.

### ADB

This app uses ADB to retrieve media playback information from WSA. ADB communication is local only (communication with WSA on the same PC) and no data is sent to external servers.

### Discord Rich Presence

This is the main feature of this app. **Only when Discord Rich Presence is enabled in settings**, the currently playing media information is sent to Discord via the Discord client and displayed as a Discord activity.

Data sent:

- Media title (song title / video title)
- Artist name
- Name of the app currently playing
- Thumbnail image URL
- Playback position and duration

Depending on your Discord settings, the displayed content may be visible to other users (e.g., friends). For information on how Discord handles your data, please refer to [Discord's Privacy Policy](https://discord.com/privacy).

## 4. Optional Third-Party Integrations

This app provides integration features for specific apps through a plugin architecture (`ArtworkResolver`). **These features only operate while the corresponding app is playing, and data is sent to external servers only at that time.**

| Integrated app | Purpose | Data sent | Destination | When sent |
|---|---|---|---|---|
| vocacolle (`jp.nicovideo.nicobox`) | Retrieve thumbnail image | Media title (song title / video title) | Niconico Search API (snapshot.search.nicovideo.jp) | Only while playing |

Resolved thumbnails are not saved as image files; only the URL is held temporarily in memory within the app and sent to Discord Rich Presence. The image itself is not downloaded or stored by this app; it is retrieved and displayed by Discord.

### Future Additions to Integrations

New third-party integrations may be added in the future due to the plugin architecture described above. If an integration is added, this policy will be updated, and the principle that "data is only transmitted while the corresponding feature is in use" will always be maintained.

## 5. Local Data Storage and Deletion

This app stores the following data locally only.

| Data | Location | Contents |
|---|---|---|
| Configuration file | `%APPDATA%\wsa-rpc-bridge\config.json` | App settings (auto-start, cache settings, etc.) |
| APK cache | `%LOCALAPPDATA%\wsa-rpc-bridge\ApkCache` | Temporary files for resolving app names |

These data are not automatically deleted when the app is uninstalled. To delete them, manually remove the `wsa-rpc-bridge` folders listed above.

## 6. Contact

If you have any questions regarding this policy, please contact:

- Developer: hu-ja-ja
- Mail: `hujaja.jp@gmail.com`

## 7. Changes to This Policy

This policy may be revised in response to feature additions/changes or changes in applicable laws. When revised, this page will be updated along with the effective date.

Changes may take effect without prior notice to you.
