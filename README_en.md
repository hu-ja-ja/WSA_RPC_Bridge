# WSA RPC Bridge

[English](README_en.md) | [日本語](README.md)

An app (Windows desktop / Android) that retrieves media playback information playing on WSA (Windows Subsystem for Android) or Android devices and displays it on Discord Rich Presence.

## Screenshots

![GUI](img/GUI.png)

![RPC](img/RPC.png)

## Quick Start

### Desktop (WSA)

1. Download the installer from [Releases](https://github.com/hu-ja-ja/WSA_RPC_Bridge/releases/latest)
2. Install and launch
3. Play music on WSA — it shows up on Discord automatically

> Developer mode must be enabled on WSA.

### Android

1. Download the APK from [Releases](https://github.com/hu-ja-ja/WSA_RPC_Bridge/releases/latest)
2. Install and grant notification access

> Requires Android 7.0+. See [Android version notes](docs/android-versions.md).

## Features

- **Auto-detect** — Catches whatever's playing on WSA or Android
- **Discord display** — Shows song info as a Rich Presence in real time
- **Cross-platform** — Works on desktop (WSA) and Android
- **Whitelist** — Pick exactly which apps to detect from (Android only)
- **No login required** — No need to log in to Discord through this app
- **Zero config** — Install, launch, done

### Notes

- **Desktop**: Windows 11 or later is required (WSA is Windows 11 only). Developer mode must be enabled on the WSA side. [WSABuilds](https://github.com/MustardChef/WSABuilds) is recommended for installing WSA.
- **Android**: Android 7.0+ is required and notification access permission must be granted. For version-dependent specs and recommended notification access settings, see [Android version notes](docs/android-versions.md) (Japanese only).

## Tech Stack

| Layer          | Tech                                      |
|----------------|-------------------------------------------|
| Frontend       | SolidJS + [Kobalte](https://kobalte.dev/) + Vite                  |
| Backend        | Rust / Tauri v2                           |
| Android Native | Kotlin (Notification Access / JNI Bridge) |

## Development

Tools are managed with [mise](https://mise.jdx.dev/).

```pwsh
mise trust        # First time only
mise install      # Installs Node / pnpm / Rust / Perl / Java
mise run deps     # Installs JS dependencies
mise run dev      # Tauri + Vite dev
mise run build    # Release build
```

See the [development command reference](docs/development.md) (Japanese only) for the full list of commands and their arguments.

## FAQ

### Will you be support for songs played using software on Windows?

No. This app is focused on WSA / Android media only. For Windows native music detection.

### Do I need to log in to Discord?

Both the desktop and Android versions require a Discord client or app that you're already logged into, but you do not need to log in to Discord within this app itself.

## Acknowledgments

### Inspiration

- [Kizzy](https://github.com/dead8309/Kizzy)

### Key Crates

- [adb_client](https://github.com/cocool97/adb_client)
- [apk-info](https://github.com/delvinru/apk-info)
- [discord-rich-presence](https://github.com/vionya/discord-rich-presence)

## Privacy Policy

Please see the [Privacy Policy](PRIVACY_POLICY_en.md) and [Terms of Service](TERMS_OF_SERVICE_en.md).

## License

Copyright (C) 2026 hu-ja-ja

[MPL-2.0](LICENSE)

Third-party licenses can be found under the licenses tab in the sidebar of the app.
