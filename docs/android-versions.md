# Android バージョン対応メモ

Android 版は **Android 7.0 (API 24)** 以上を対象としています（`src-tauri/gen/android/app/build.gradle.kts` の `minSdk = 24`）。
Android の仕様はバージョンによって大きく変わるため、対応状況を以下にまとめます。

## バージョンで変わる仕様と対応状況

| 仕様 | 必要Android (API) | 対応状況 |
|---|---|---|
| 通知チャネル（チャネルなし通知は投稿不可） | 8.0 (26) | `MediaInfoService.createChannel()` が API 26 未満では早期 return。7.x では従来方式で通知 |
| `startForegroundService()` の必須化 | 8.0 (26) | `MediaInfoService.start()` が API 26 以降は `startForegroundService()`、未満は `startService()` に分岐 |
| フォアグラウンドサービスの型 (`mediaPlayback`) | 10 (29) | `ServiceCompat.startForeground()` が API 29 以降のみ型を指定、未満は型なし |
| `FOREGROUND_SERVICE_MEDIA_PLAYBACK` 権限の必須化 | 14 (34) | Manifest に常時宣言（14 未満では無視される） |
| `POST_NOTIFICATIONS` ランタイム権限 | 13 (33) | `MainActivity.hasNotificationPermission()` が 13 未満では常に許可扱い。13+ で未許可時は FGS 通知が非表示になり、許可後にサービスを再起動して再投稿 |
| 署名証明書の取得 API | 9 (28) | `SignatureBridge` が API 28 以降は `GET_SIGNING_CERTIFICATES`、未満は非推奨の `GET_SIGNATURES` に分岐 |
| パッケージ可視性 (`<queries>`) | 11 (30) | Manifest に宣言。11 未満では不要（自動で全パッケージが見える） |
| 通知アクセス設定画面のカテゴリ表示 | 14 (34) | 14+ のみ「リアルタイム / 会話 / 通知 / 消音」が表示される。14 未満ではアプリの切替のみ表示 |

### WebView について

UI は Android システムの WebView（Tauri）で描画されます。Android 7.x でも Play ストア経由で WebView を更新できますが、古い WebView のままでは最新の JavaScript 機能（ES Module など）が動かず UI が表示されない可能性があります。

## 通知アクセス権限の推奨設定

通知アクセス権限の画面では、アプリの権限（サービス自体）に加えて以下のオプションが表示されます。本アプリは通知本文を読み取らず、メディア情報の検出にのみ使用するため、次の設定で問題ありません。

- **リアルタイム**: ON（推奨） — 音楽アプリの再生通知（進行バー付き）の検出に使用
- **会話 / 通知 / 消音**: OFF — 通知本文を読み取らないため不要。OFF にすると OS が配信する通知が絞られ、より安全です

> **注意**: このカテゴリ分けは **Android 14 (API 34) 以降の設定画面**で表示されます。それ未満の Android ではアプリの許可切替のみが表示され、カテゴリは出ません。
