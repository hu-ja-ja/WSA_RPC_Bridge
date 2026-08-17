# WSA RPC Bridge

[English](README_en.md) | [日本語](README.md)

WSA (Windows Subsystem for Android) や Android デバイス上で再生されているメディア情報を取得し、Discord Rich Presence に表示するアプリ（Windows デスクトップ / Android 対応）。

## 注意

- **デスクトップ版**: WSA 側で開発者モードを有効にしている必要があります。
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

## 開発環境

ツールは [mise](https://mise.jdx.dev/) で管理しています。

### Windows

1. [Visual Studio Build Tools](https://visualstudio.microsoft.com/ja/downloads/) をインストールし、**C++によるデスクトップ開発** ワークロードと Windows SDK を含める（以下のコマンドでも可）
2. `mise trust`（初回のみ）
3. `mise install` で Node / pnpm / Rust / Perl / Java を導入
4. `mise run deps` で JS 依存をインストール

```pwsh
winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.Windows11SDK.26100"
mise trust
mise install
mise run deps
```

### Linux

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libssl-dev libayatana-appindicator3-dev librsvg2-dev
mise trust
mise install
mise run deps
```

- Perl は mise が管理します（Windows: Strawberry Perl / Linux・macOS: relocatable-perl）
- Android SDK は `$ANDROID_HOME`（未設定なら `~/Android/Sdk`）を自動検出し、NDK のクロスコンパイル設定も mise が OS に合わせて生成します

## ビルド

```bash
mise run build # リリースビルド
```

## 開発

```bash
mise run dev    # Tauri + Vite dev
mise run lint   # oxlint
mise tauri ...  # tauri CLI へのパススルー（例: mise tauri android dev）
```

## テスト

```bash
mise run test              # ユニットテスト（パーサーなど）
mise run test -- --ignored # WSA 実機が必要な結合テスト
```

## Android

```bash
mise tauri android dev    # Android アプリを実行
mise tauri android build  # APK / AAB をビルド
```

tauri CLI が Android SDK を自動検出します。

- SDK が既定の場所（`%LOCALAPPDATA%\Android\Sdk`）にあればそのまま使用
- 無ければ対話式で cmdline-tools / platform-tools / `platforms;android-36` / `ndk;29.0.13846066` を自動インストール
- NDK は `$ANDROID_HOME\ndk` 内の最新バージョンを使用
- `ANDROID_HOME` / `NDK_HOME` の環境変数は不要（SDK を独自の場所に置く場合のみ設定）
- JDK は mise の `java = "21"` で管理

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
