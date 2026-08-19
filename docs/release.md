# リリース方針

GitHub Actions の `workflow_dispatch` からリリースを生成する。

## リリースフロー

1. Discord Developer Portal で Social SDK のダウンロードURLを生成する（約1時間で失効するGCS署名付きURL）。
2. GitHub の Actions タブから `Release` ワークフローを手動起動し、以下を入力する:
   - `version`: アプリのバージョン（例 `0.4.0`）→ タグ `v{version}` としてリリース
   - `sdk_url`: 上記の期限付きURL
3. ワークフローが以下を実行する:
   - バージョンを `tauri.conf.json` / `Cargo.toml` に同期
   - `sdk_url` から Social SDK を取得し `src-tauri/gen/android/app/libs/discord_partner_sdk.aar` へ配置
   - Android APK（`aarch64`）+ Windows MSI をビルド
   - CHANGELOG.md から該当バージョンのセクションを抽出し、リリース本文とする（セクションが無い場合はコミット一覧でフォールバック）
   - リリースを自動作成し、成果物を添付
   - `update.json` を GitHub Pages にデプロイ（デスクトップのアップデーター用）

## Social SDK の取り扱い

- **公開リポジトリに SDK バイナリをコミットしない。**
  利用規約 2.b は SDK 自体の再配布・コピーを禁止している。アプリへの同梱は規約 2.a でライセンス付与されているが、単体での公開は再配布に該当する。
- **期限付きURLを恒久保管しない。** ポータルが発行する失効URLを迂回してバイナリを private repo などに永続化しない。
- 毎回ポータルで新しくURLを生成し、取得→即ビルド→アプリに同梱して公開する。SDKは Discrord CDN → CI → 配布アプリ の流れのみを通り、どこにも残らない。
- `sdk_url` は workflow_dispatch の入力として渡す。ログ流出対策として `::add-mask::` を適用する（失効が早いためリスクは小さい）。

## 変更ログ

- `CHANGELOG.md` を手書きで維持する（追加/変更/削除 のセクション形式）。
- ワークフローは該当バージョンの `## [version]` セクションを抽出してリリース本文に使うため、GitHub UI で本文を書く必要はない。

## 成果物

- Windows: MSI + `.sig` + `.sha256`
- Android: APK（`aarch64` のみ。エミュレータ向け ABI が必要になったら増やす）
- APK はリリース署名キーで署名され、署名証明書の SHA-256 フィンガープリントが `apk-signing-fingerprint.txt` として添付される。アプリの「アプリ概要」画面に表示されるフィンガープリントと照合することで、配布APKの真正性を確認できる。

## Android リリース署名キー

- GitHub リポジトリの Secrets に以下を設定する
  - `ANDROID_KEYSTORE_BASE64`: release keystore を Base64 化したもの
  - `ANDROID_KEYSTORE_PASSWORD` / `ANDROID_KEY_ALIAS` / `ANDROID_KEY_PASSWORD`

## CI の Android 環境

- windows-latest の `ANDROID_HOME` は標準搭載。
- NDK を sdkmanager でインストールする（`yes | sdkmanager --licenses` → `ndk;29.0.13846066`）。
- Rust の Android ターゲットは tauri CLI が自動で追加する。
- 1時間URLの失効対策として Android ビルドを先に実行する。

## 注意

- 旧 `release.yml` の `on: release: [published]` トリガーを残すと、新フローが作ったリリースで再ビルドが走るため、**置き換え時に削除する**。
