use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

use crate::i18n::tr;

pub fn setup_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", tr("tray.open"), true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", tr("tray.settings"), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", tr("tray.quit"), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &quit])?;

    let icon = app.default_window_icon().cloned()
        .expect("default window icon must be configured");

    TrayIconBuilder::new()
        .icon(icon)
        .show_menu_on_left_click(false)
        .menu(&menu)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        })
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "open" => show_main_window(app),
                "settings" => {
                    show_main_window(app);
                    let _ = app.emit("show-settings", ());
                }
                "quit" => std::process::exit(0),
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}

pub fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
