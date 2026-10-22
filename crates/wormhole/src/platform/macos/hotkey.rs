//! What `platform::hotkey` needs from macOS: Carbon's hotkeys are registered on the main thread.

pub fn on_registering_thread() -> bool {
    objc2::MainThreadMarker::new().is_some()
}
