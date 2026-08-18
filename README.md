# WSA RPC Bridge

[English](README_en.md) | [日本語](README.md)

WSA (Windows Subsystem for Android) や Android デバイス上で再生されているメディア情報を取得し、Discord Rich Presence に表示するアプリ（Windows デスクトップ / Android 対応）。

## 注意

- **デスクトップ版**: Windows 11 以上が必要です（WSA は Windows 11 専用）。WSA 側で開発者モードを有効にしている必要があります。WSA の導入には [WSABuilds](https://github.com/MustardChef/WSABuilds) を推奨します。
- **Android 版**: Android 7.0 以上が必要で、通知アクセス権限の許可が必要です。

![WSA_Config](img/WSA_Config.png)

## 機能

- 再生情報（曲タイトル、アーティスト、アルバム、再生位置、アルバムアート）の自動取得
  - デスクトップ版: WSA 上のアプリを ADB 経由で取得
  - Android 版: 通知アクセス経由で直接取得
- Discord Rich Presence に再生状況を表示
- メディアを検出するアプリの選択（ホワイトリスト）
- アプリ名の自動解決
- トレイ常駐（起動時 / 最小化時 / 閉じる時の格納を設定可能）
- 自動起動（Windows）
- 自動アップデート
- ライセンス表示タブ
- マルチ言語対応（日本語 / English）

## スクリーンショット

![GUI](img/GUI.png)

![RPC](img/RPC.png)

## 技術スタック

- **フロントエンド**: SolidJS + Kobalte + Vite（デスクトップ / Android 共通）
- **バックエンド**: Rust / Tauri v2
- **Android ネイティブ**: Kotlin（通知アクセスによるメディア取得 / JNI ブリッジ）

## 開発

ツールは [mise](https://mise.jdx.dev/) で管理しています。

```bash
mise trust        # 初回のみ
mise install      # Node / pnpm / Rust / Perl / Java を導入
mise run deps     # JS 依存をインストール
mise run dev      # Tauri + Vite dev
mise run build    # リリースビルド
```

全コマンドと引数のリファレンスは [開発コマンドリファレンス](docs/development.md) を参照。

## インストール

- **デスクトップ版**: [Releases](https://github.com/hu-ja-ja/WSA_RPC_Bridge/releases) から最新のインストーラをダウンロードして実行してください。アプリ内から自動更新もできます。
- **Android 版**: [Releases](https://github.com/hu-ja-ja/WSA_RPC_Bridge/releases) から最新の APK をダウンロードしてインストールしてください。

## 謝辞

### アイデア

- [Kizzy](https://github.com/dead8309/Kizzy)

### 主要Crates

- [adb_client](https://github.com/cocool97/adb_client)
- [discord-rich-presence](https://github.com/vionya/discord-rich-presence)
- [apk-info](https://github.com/delvinru/apk-info)

## プライバシーポリシー

[プライバシーポリシー](PRIVACY_POLICY.md) と [利用規約](TERMS_OF_SERVICE.md) をご確認ください。

## ライセンス

Copyright (C) 2026 hu-ja-ja

MPL-2.0

詳細は [LICENSE](LICENSE) ファイルを参照してください。

サードパーティライセンスは、アプリ内のサイドバーのライセンスタブから確認できます。
`mise run generate-licenses` で自動生成されています。
