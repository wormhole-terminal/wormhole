//! AppKit's side of `platform::native_settings`: the SwiftUI window of `macos/settings/`, linked as a static
//! library by `build.rs`. It exposes five C functions (`Host.swift`); text crosses as NUL-terminated UTF-8.

use std::ffi::{CStr, CString, c_char};
use std::sync::OnceLock;

// SAFETY (all five): defined in `macos/settings/Sources/SettingsUI/Host.swift` with `@_cdecl` and the same
// signatures; each runs on the main actor, which every caller here is on (the app's UI thread).
#[allow(unsafe_code)]
unsafe extern "C" {
    fn wh_settings_open(page: *const c_char, behind: bool, handler: extern "C" fn(*const c_char));
    fn wh_settings_update(json: *const c_char);
    fn wh_settings_is_open() -> bool;
    fn wh_settings_shot(path: *const c_char) -> bool;
    fn wh_settings_look(out: *mut c_char, cap: usize) -> bool;
}

/// Where the window's events go; set on the first open, and the same function every time.
static HANDLER: OnceLock<fn(&str)> = OnceLock::new();

pub fn available() -> bool {
    true
}

pub fn open(page: &str, behind: bool, on_event: fn(&str)) {
    let _ = HANDLER.set(on_event);
    let Ok(page) = CString::new(page) else { return };
    // SAFETY: `page` is a valid NUL-terminated string that outlives the call (Swift copies it before returning);
    // `trampoline` is a plain function that lives for the whole program.
    #[allow(unsafe_code)]
    unsafe {
        wh_settings_open(page.as_ptr(), behind, trampoline);
    }
}

pub fn update(snapshot: &str) {
    let Ok(snapshot) = CString::new(snapshot) else { return };
    // SAFETY: a valid NUL-terminated string that outlives the call; Swift parses it before returning.
    #[allow(unsafe_code)]
    unsafe {
        wh_settings_update(snapshot.as_ptr());
    }
}

pub fn is_open() -> bool {
    // SAFETY: takes and returns plain values.
    #[allow(unsafe_code)]
    unsafe {
        wh_settings_is_open()
    }
}

/// Swift's callback: the event text is valid only for the call, so it is handed on as a `&str` the handler copies.
extern "C" fn trampoline(event: *const c_char) {
    if event.is_null() {
        return;
    }
    // SAFETY: Swift passes a NUL-terminated string (`withCString`) that lives until this function returns.
    #[allow(unsafe_code)]
    let text = unsafe { CStr::from_ptr(event) };
    if let (Some(handler), Ok(text)) = (HANDLER.get(), text.to_str()) {
        handler(text);
    }
}

/// The window's content written as a PNG at `path`; whether it was.
pub fn shot(path: &str) -> bool {
    let Ok(path) = CString::new(path) else { return false };
    // SAFETY: a valid NUL-terminated string that outlives the call.
    #[allow(unsafe_code)]
    unsafe {
        wh_settings_shot(path.as_ptr())
    }
}

/// The window's look as it is in force (devctl): the flags AppKit holds, `None` while the window is closed.
pub fn look() -> Option<String> {
    let mut buf = [0 as c_char; 512];
    // SAFETY: `buf` is writable for the `cap` given; Swift writes at most `cap - 1` bytes and a NUL.
    #[allow(unsafe_code)]
    let ok = unsafe { wh_settings_look(buf.as_mut_ptr(), buf.len()) };
    if !ok {
        return None;
    }
    // SAFETY: Swift wrote a NUL-terminated string into `buf`.
    #[allow(unsafe_code)]
    let text = unsafe { CStr::from_ptr(buf.as_ptr()) };
    text.to_str().ok().map(str::to_string)
}
