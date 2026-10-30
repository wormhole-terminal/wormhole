//! `platform::dev_tools` on macOS: System Settings → Privacy & Security → Developer Tools.

use gpui_kit::*;

pub fn open_settings(cx: &App) {
    cx.open_url("x-apple.systempreferences:com.apple.preference.security?Privacy_DevTools");
}
