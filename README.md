# WSA RPC Bridge

[English](README_en.md) | [日本語](README.md)

WSA (Windows Subsystem for Android) や Android デバイス上で再生されているメディア情報を取得し、Discord Rich Presence に表示するアプリ (Windows デスクトップ / Android 対応) 。

## スクリーンショット

![GUI](img/GUI.png)

![RPC](img/RPC.png)

## クイックスタート

### デスクトップ版 (WSA)

1. [Releases](https://github.com/hu-ja-ja/WSA_RPC_Bridge/releases/latest) からインストーラをダウンロード
2. インストール → 起動
3. WSA で音楽を再生すれば、自動で Discord に反映される

> WSA の開発者モードの有効化が必要です。

### Android 版

1. [Releases](https://github.com/hu-ja-ja/WSA_RPC_Bridge/releases/latest) から APK をダウンロード
2. インストール → 通知アクセスを許可

> Android 7.0 以上が必要です。[バージョン対応メモ](docs/android-versions.md) も参照してください。

## 機能

- **自動検出** — WSA / Android で再生中の曲を自動でキャッチ
- **Discord 表示** — リッチプレゼンスに曲情報をリアルタイム表示
- **マルチプラットフォーム** — デスクトップ (WSA) と Android 両対応
- **ホワイトリスト** — 検出したいアプリだけ選べる (Android版のみ)
- **ログイン不要** — 本アプリで Discord にログインする必要はありません
- **かんたん設定** — インストールして起動するだけ、あとは自動

### 注意

- **デスクトップ版**: Windows 11 以上が必要です (WSA は Windows 11 専用) 。WSA 側で開発者モードを有効にしている必要があります。WSA の導入には [WSABuilds](https://github.com/MustardChef/WSABuilds) を推奨します。
- **Android 版**: Android 7.0 以上が必要で、通知アクセス権限の許可が必要です。バージョン依存の仕様と通知アクセス権限の推奨設定は [Android バージョン対応メモ](docs/android-versions.md) を参照してください。

## 技術スタック

| レイヤー           | 技術                                  |
|--------------------|---------------------------------------|
| フロントエンド     | SolidJS + [Kobalte](https://kobalte.dev/) + Vite              |
| バックエンド       | Rust / Tauri v2                       |
| Android ネイティブ | Kotlin（通知アクセス / JNI ブリッジ） |

## 開発

ツールは [mise](https://mise.jdx.dev/) で管理しています。

```pwsh
mise trust        # 初回のみ
mise install      # Node / pnpm / Rust / Perl / Java を導入
mise run deps     # JS 依存をインストール
mise run dev      # Tauri + Vite dev
mise run build    # リリースビルド
```

全コマンドと引数のリファレンスは [開発コマンドリファレンス](docs/development.md) を参照。

## FAQ

### Windows 上のソフトウェアで再生している曲に対応する予定はありますか？

いいえ。本アプリは WSA / Android 上のメディアに特化しています。

### Discord へのログインは必要ですか？

デスクトップ版、Android版ともに、ログイン済みの Discord クライアント/アプリが必要ですが、本アプリ自体でDiscordにログインする必要はありません。

## 謝辞

### アイデア

- [Kizzy](https://github.com/dead8309/Kizzy)

### 主要Crates

- [adb_client](https://github.com/cocool97/adb_client)
- [apk-info](https://github.com/delvinru/apk-info)
- [discord-rich-presence](https://github.com/vionya/discord-rich-presence)

## プライバシーポリシー

[プライバシーポリシー](PRIVACY_POLICY.md) と [利用規約](TERMS_OF_SERVICE.md) をご確認ください。

## ライセンス

Copyright (C) 2026 hu-ja-ja

[MPL-2.0](LICENSE)

サードパーティライセンスはアプリ内のライセンスタブから確認できます。
