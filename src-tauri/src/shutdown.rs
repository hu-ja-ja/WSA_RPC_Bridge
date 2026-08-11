use std::ffi::c_void;
use std::sync::OnceLock;

use tauri::{AppHandle, WebviewWindow};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DefWindowProcW, GWLP_WNDPROC, SetWindowLongPtrW, WM_ENDSESSION,
};

type WndProc = unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize;

static APP: OnceLock<AppHandle> = OnceLock::new();
static ORIG_WNDPROC: OnceLock<isize> = OnceLock::new();

unsafe extern "system" fn wnd_proc(hwnd: *mut c_void, msg: u32, wparam: usize, lparam: isize) -> isize {
    // WM_ENDSESSION with wparam != 0 means Windows is shutting down. Exit right
    // away so the JS poller stops and no adb.exe is spawned during the loader
    // teardown (which would surface as adb.exe 0xc0000142).
    if msg == WM_ENDSESSION && wparam != 0 {
        log::info!("Windows shutdown detected; exiting immediately");
        if let Some(app) = APP.get() {
            app.exit(0);
        }
    }

    let orig = ORIG_WNDPROC.get().copied().unwrap_or(0);
    if orig != 0 {
        let orig: WndProc = std::mem::transmute(orig);
        CallWindowProcW(Some(orig), hwnd, msg, wparam, lparam)
    } else {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

pub fn install(app: &AppHandle, window: &WebviewWindow) {
    if APP.set(app.clone()).is_err() {
        return;
    }
    if let Ok(hwnd) = window.hwnd() {
        let new_proc = wnd_proc as WndProc;
        let prev = unsafe { SetWindowLongPtrW(hwnd.0, GWLP_WNDPROC, new_proc as usize as isize) };
        let _ = ORIG_WNDPROC.set(prev);
    }
}