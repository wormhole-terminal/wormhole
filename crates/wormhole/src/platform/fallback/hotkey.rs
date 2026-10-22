//! `platform::hotkey` elsewhere: `global-hotkey` registers from any thread.

pub fn on_registering_thread() -> bool {
    true
}
