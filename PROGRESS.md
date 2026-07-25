# 進捗

## Phase 1: ADB Core — dumpsys media_session の取得・パース ✅

- [x] `adb_client` crate を使って WSA (127.0.0.1:58526) に接続
- [x] `dumpsys media_session` を実行し出力を取得
- [x] 出力をパースして `MediaInfo`（title, artist, album, position, is_playing）を抽出
- [x] Tauri コマンド `get_media_info` を追加
- [x] フロントエンド (SolidJS) に接続状態と再生情報を表示
- [x] パーサーのユニットテスト (実WSA出力で検証済み)

## Phase 2: Discord RPC Core — 最小サンプル表示 ✅

- [x] `discord-rich-presence` crate を導入 (Pure Rust, IPC経由)
- [x] DiscordRpc::connect / update_presence / disconnect の実装 (専用IPCスレッド)
- [x] MediaInfo を Discord アクティビティに反映 (details=title, state=artist, timestamps=位置)
- [x] Tauri コマンド: connect_discord, disconnect_discord, update_discord_presence, get_discord_status
- [x] `get_adb_status` のバグ修正 (誤って discord の状態を見ていた)
- [x] フロントエンド: Discord 接続状態インジケータ、メディア取得時に自動で presence 更新
- [x] アプリ起動時に Discord IPC へ自動接続を試行

## Phase 3: 統合・UI完成 — TODO

- [ ] ポーリングループで自動更新
- [ ] RPC オン/オフ トグル (Kobalte)
- [ ] エラーハンドリング改善
