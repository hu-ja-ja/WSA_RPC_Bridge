# 進捗

## Phase 1: ADB Core — dumpsys media_session の取得・パース ✅

- [x] `adb_client` crate を使って WSA (127.0.0.1:58526) に接続
- [x] `dumpsys media_session` を実行し出力を取得
- [x] 出力をパースして `MediaInfo`（title, artist, album, position, is_playing）を抽出
- [x] Tauri コマンド `get_media_info` を追加
- [x] フロントエンド (SolidJS) に接続状態と再生情報を表示
- [x] パーサーのユニットテスト (実WSA出力で検証済み)

## Phase 2: Discord RPC Core — TODO

- [ ] Discord Social SDK (C++) を cxx 経由で呼び出す FFI 層
- [ ] DiscordRpc::connect / update_presence / disconnect の実装
- [ ] MediaInfo を Discord アクティビティに反映

## Phase 3: 統合・UI完成 — TODO

- [ ] ポーリングループで自動更新
- [ ] RPC オン/オフ トグル (Kobalte)
- [ ] エラーハンドリング改善
