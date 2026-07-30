use std::sync::OnceLock;

fn lang() -> &'static str {
    static LANG: OnceLock<&'static str> = OnceLock::new();
    LANG.get_or_init(|| {
        sys_locale::get_locale()
            .map(|l| if l.starts_with("ja") { "ja" } else { "en" })
            .unwrap_or("en")
    })
}

pub fn tr(key: &str) -> String {
    match lang() {
        "ja" => tr_ja(key),
        _ => tr_en(key),
    }
}

fn tr_ja(key: &str) -> String {
    match key {
        "tray.open" => "開く".into(),
        "tray.settings" => "設定".into(),
        "tray.quit" => "終了".into(),
        "adb.connect_failed" => "WSAデバイスへの自動ADB接続に失敗しました。WSAが開発者モードで実行されていることを確認してください。".into(),
        "adb.dumpsys_failed" => "dumpsys media_sessionの実行に失敗しました".into(),
        "adb.utf8_decode_failed" => "dumpsys出力をUTF-8としてデコードできませんでした".into(),
        "adb.no_session" => "アクティブなメディアセッションが見つかりません".into(),
        _ => key.into(),
    }
}

fn tr_en(key: &str) -> String {
    match key {
        "tray.open" => "Open".into(),
        "tray.settings" => "Settings".into(),
        "tray.quit" => "Quit".into(),
        "adb.connect_failed" => "Automatic ADB connection to WSA device (127.0.0.1:58526) failed. Ensure WSA is running with developer mode enabled.".into(),
        "adb.dumpsys_failed" => "Failed to execute dumpsys media_session".into(),
        "adb.utf8_decode_failed" => "Failed to decode dumpsys output as UTF-8".into(),
        "adb.no_session" => "No active media session found".into(),
        _ => key.into(),
    }
}
