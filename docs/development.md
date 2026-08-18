# 開発コマンドリファレンス

ツールの導入・環境構築の手順は [README](../README.md) を参照。コマンドは [mise](https://mise.jdx.dev/) のタスクとして定義されており、mise が管理するツール（Node / pnpm / Rust / Java / Perl など）が自動で PATH に入る。

`dev` / `build` / `tauri` タスクは [Infisical](https://infisical.com/) の `infisical run` を経由して実行される。Infisical が管理しているのはリリース署名用の秘密鍵（`TAURI_SIGNING_PRIVATE_KEY`、Android keystore など）のみで、通常の開発では必須ではない。未ログインの場合は下記の [代替コマンド](#infisical-未ログイン時の代替コマンド) を利用する。

## コマンド一覧

| コマンド | 実体 | 用途 |
|---|---|---|
| `mise run deps` | `pnpm install` | JS 依存のインストール |
| `mise run dev` | `infisical run -- pnpm tauri dev` | デスクトップ版を開発モードで起動 |
| `mise run build` | `infisical run -- pnpm tauri build` | デスクトップ版リリースビルド |
| `mise run lint` | `pnpm lint`（oxlint） | フロントエンドの静的解析 |
| `mise run test` | `cargo test`（src-tauri 内） | Rust ユニットテスト |
| `mise run android-test` | `gradlew.bat test`（`src-tauri/gen/android` 内） | Android ユニットテスト（Robolectric） |
| `mise run tauri` | `infisical run -- pnpm tauri` | tauri CLI へのパススルー |
| `mise run generate-licenses` | `pnpm generate-licenses` | サードパーティライセンスの再生成 |
| `pnpm preview` | `vite preview` | ビルド成果物のローカル確認 |

## 各コマンドの詳細と引数

`mise run <task> -- <args>` の形式で引数を渡せる（`--` 以降がタスクのコマンドにそのまま渡る）。`mise run tauri ...` は `mise tauri ...` と略記できる。

### Infisical 未ログイン時の代替コマンド

`dev` / `build` / `tauri` は `infisical run` を挟むため、ログインしていないと失敗する。その場合は pnpm を直接実行すればよい。`pnpm tauri ...` は `--` 不要で引数がそのまま tauri CLI に渡る。

| mise タスク | 代替コマンド |
|---|---|
| `mise run dev [引数]` | `pnpm tauri dev [引数]` |
| `mise run build [引数]` | `pnpm tauri build [引数]` |
| `mise tauri <サブコマンド> [引数]` | `pnpm tauri <サブコマンド> [引数]` |

```bash
pnpm tauri dev                              # 通常起動
pnpm tauri dev --release                    # リリースモード
pnpm tauri build --bundles msi              # MSI のみビルド
pnpm tauri android build --apk -t aarch64   # Android APK（aarch64）
```

- runner（cargo など）へ引数を渡す場合のみ `--` が必要: `pnpm tauri dev -- -- [runnerArgs]`
- `deps` / `lint` / `test` / `android-test` / `generate-licenses` は infisical を使わないため、ログイン状態に関係なく `mise run` で実行できる
- **注意**: 未ログインビルドには署名用秘密鍵が注入されないため、署名済みの成果物（署名付き APK / アップデーター署名）は生成されない。開発用の `dev` 実行には影響しない。リリース署名が必要な場合はログインして `mise run build` を使う。

### `mise run dev` — デスクトップ版の開発

```bash
mise run dev                 # 通常起動（hot-reload 有効）
mise run dev -- --release    # リリースモードで実行
mise run dev -- --no-watch   # ファイル監視を無効化
```

`tauri dev` の主要引数:

| 引数 | 説明 |
|---|---|
| `--release` | リリースモードで実行 |
| `-t, --target <triple>` | ターゲット triple を指定 |
| `-f, --features <feat>` | cargo の features を有効化 |
| `--no-watch` | ファイル監視を無効化 |
| `--no-dev-server` | 組み込み dev server を無効化 |
| `--port <port>` | 静的ファイル用 dev server のポート（既定 1430） |
| `-c, --config <path>` | 追加設定ファイルをマージ |
| `-- [runnerArgs] -- [appArgs]` | `--` で区切ってアプリに引数を渡す |

起動時、`build.beforeDevCommand`（`pnpm generate-licenses && pnpm dev`）が先に実行され、ライセンス生成 → Vite dev server（`http://localhost:5173`）が立ち上がる。

### `mise run build` — デスクトップ版のリリースビルド

```bash
mise run build                    # MSI + NSIS を生成
mise run build -- --bundles msi   # MSI のみ
mise run build -- --no-bundle     # バンドルなし（バイナリのみ）
mise run build -- -d              # デバッグビルド
mise run build -- -t aarch64-pc-windows-msvc  # クロスコンパイル
```

`tauri build` の主要引数:

| 引数 | 説明 |
|---|---|
| `-b, --bundles <msi\|nsis>` | 生成するバンドルの種類 |
| `--no-bundle` | バンドル生成をスキップ |
| `-d, --debug` | デバッグフラグ付きでビルド |
| `-t, --target <triple>` | ターゲット triple を指定 |
| `-f, --features <feat>` | cargo の features を有効化 |
| `--no-sign` | コード署名をスキップ |
| `-c, --config <path>` | 追加設定ファイルをマージ |
| `--ci` | プロンプトを出さず実行 |

### `mise run lint` — 静的解析

```bash
mise run lint                     # 通常実行
mise run lint -- --fix            # 自動修正
mise run lint -- --quiet          # warning を非表示
mise run lint -- -D correctness   # カテゴリ単位で指定（-A で許可 / -W で警告 / -D でエラー）
mise run lint -- -f json          # 出力形式（default / json / junit / github など）
mise run lint -- --max-warnings 10
mise run lint -- src/components/foo.tsx  # ファイル/パスを指定
```

### `mise run test` — Rust テスト

```bash
mise run test                        # ユニットテスト（パーサーなど）
mise run test -- -- --ignored        # WSA 実機が必要な結合テスト
mise run test -- parser              # テスト名でフィルタ
mise run test -- -- --nocapture      # println 出力を表示
```

- 1つ目の `--` は mise が取り除いてタスクへ渡す。`cargo test` のテストバイナリへの引数（`--ignored` など）は cargo 自身の `--` も必要なので `-- --` と 2 重になる。
- `--ignored` のテストは `src-tauri/tests/adb_test.rs` で、WSA が開発者モードで起動し ADB サーバー（127.0.0.1:58526）が応答する環境が必要。

### `mise run android-test` — Android ユニットテスト

```bash
mise run android-test                # 全テスト
mise run android-test -- --tests "クラス名"  # テストを絞り込み
```

- `src-tauri/gen/android` で `gradlew.bat test` を実行（Robolectric）。
- **fresh checkout では事前に `mise run tauri android init` が必要**（`gen/android` の gradle ファイルと生成 Kotlin コードが gitignore されていて存在しないため）。CI は独自の生成ステップで対応している。

### `mise run tauri ...` — tauri CLI パススルー

```bash
mise tauri info                    # 環境情報（Rust / Node / 設定）の確認
mise tauri android init            # Android プロジェクトを初期化（fresh checkout 時に必要）
mise tauri icon <png>              # アイコン生成
mise tauri signer generate         # アップデーター用署名キー生成
```

#### `tauri android dev` — Android 版の開発

```bash
mise tauri android dev                  # 接続中のデバイス/エミュレータで実行
mise tauri android dev -- -o            # Android Studio を開く
mise tauri android dev -- --host        # 実機向けに公開アドレスを使う
mise tauri android dev <device-id>      # デバイスを指定
mise tauri android dev -- --release     # リリースモード
```

#### `tauri android build` — APK / AAB のビルド

```bash
mise tauri android build --apk                # APK を生成（全 ABI）
mise tauri android build --apk -t aarch64     # aarch64 のみ（リリースで使用）
mise tauri android build --aab                # AAB を生成
mise tauri android build --split-per-abi      # ABI ごとに分割
mise tauri android build -- -o                # 完了後 Android Studio を開く
```

| 引数 | 説明 |
|---|---|
| `--apk` / `--aab` | 生成形式（既定は両方） |
| `-t, --target <aarch64\|armv7\|i686\|x86_64>` | 対象 ABI（既定は全部） |
| `--split-per-abi` | ABI ごとに APK/AAB を分割 |
| `-d, --debug` | デバッグビルド |
| `-o, --open` | ビルド後に Android Studio を開く |
| `--ci` | プロンプトを出さず実行 |

### `mise run generate-licenses` — ライセンス生成

```bash
mise run generate-licenses
```

- `scripts/generate-licenses.mjs` が JS と Rust（cargo-about）の依存を収集し、アプリ内ライセンスタブのデータを再生成する。`tauri dev` / `tauri build` の `beforeDevCommand` / `beforeBuildCommand` でも自動実行される。

### `pnpm preview` — ビルド成果の確認

```bash
pnpm preview                    # dist をローカルサーバーで配信
pnpm preview -- --port 8080     # ポート指定
pnpm preview -- --open          # ブラウザで自動オープン
```

## CI との対応

ローカルで CI 相当の検証を行う場合、[ci.yml](../.github/workflows/ci.yml) は以下を順に実行している:

1. `mise run deps`
2. `mise run generate-licenses`
3. `mise run lint`
4. `mise run build`
5. `mise run test`
6. `mise run android-test`（前述の gradle 生成ステップを経由）

リリース（[release.yml](../.github/workflows/release.yml) / [release.md](release.md)）で使われるビルドコマンド:

```bash
pnpm tauri android build --apk --target aarch64   # Android APK（aarch64）
pnpm tauri build --bundles msi                    # Windows MSI
```

## 関連ドキュメント

- [リリース方針](release.md)