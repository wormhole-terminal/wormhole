//! AppKit's side of `platform::sound`.

pub fn beep() {
    objc2_app_kit::NSBeep();
}
