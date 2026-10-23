//! The global hotkey (Settings → Menu bar): registered with the system as the shown profile sets it, held
//! off while the settings window records a new one, and reporting why it didn't register. It is a global of
//! its own, set at launch with the sender the status item's menu commands arrive on, so the settings
//! window (below the status item) can hold it off and read its error.

use gpui_kit::*;
use wormhole_core::storage::paths::ProfileId;

use crate::menubar::MenuCommand;
use crate::platform::hotkey::Hotkey;

/// The hotkey and what it was last applied as.
pub(crate) struct GlobalHotkey {
    tx: smol::channel::Sender<MenuCommand>,
    hotkey: Option<Hotkey>,
    /// The hotkey setting last applied, so a bad spec isn't retried every tick.
    applied: Option<String>,
    /// The profile that setting is the shown one's of: after a switch the hotkey is registered again as the
    /// new profile sets it.
    profile: Option<ProfileId>,
    /// Held off while the settings window records a new one: the keys pressed then reach
    /// the recorder, the current hotkey's included.
    paused: bool,
    error: Option<String>,
}

impl Global for GlobalHotkey {}

/// Sets the hotkey up (registering nothing yet): a press arrives as [`MenuCommand::Hotkey`] on `tx`'s
/// receiver.
pub(crate) fn install(tx: smol::channel::Sender<MenuCommand>, cx: &mut App) {
    cx.set_global(GlobalHotkey {
        tx,
        hotkey: None,
        applied: None,
        profile: None,
        paused: false,
        error: None,
    });
}

/// Every tick: registers `spec` (the setting, trimmed) for the shown `profile` if that is not what is
/// registered. Whether it changed anything, so the settings window can show the result.
pub(crate) fn sync(spec: &str, profile: &ProfileId, cx: &mut App) -> bool {
    if !cx.has_global::<GlobalHotkey>() {
        return false;
    }
    let state = cx.global_mut::<GlobalHotkey>();
    if state.profile.as_ref() != Some(profile) {
        state.profile = Some(profile.clone());
        state.applied = None;
    }
    if state.applied.as_deref() == Some(spec) {
        return false;
    }
    state.apply(spec);
    true
}

/// Holds the hotkey off while its field in Settings records, or puts it back. Nothing, if the app has no
/// hotkey yet.
pub(crate) fn pause(paused: bool, cx: &mut App) {
    if !cx.has_global::<GlobalHotkey>() {
        return;
    }
    let state = cx.global_mut::<GlobalHotkey>();
    if state.paused == paused {
        return;
    }
    state.paused = paused;
    if let Some(spec) = state.applied.clone() {
        state.apply(&spec);
    }
}

/// Why the hotkey set now didn't register, if it didn't.
pub(crate) fn error(cx: &App) -> Option<String> {
    cx.try_global::<GlobalHotkey>()?.error.clone()
}

/// The hotkey as the menus show it: the spec in force, while it is registered or held off by a recording
/// (shown as it would be, as background mode's, which holds nothing, does).
pub(crate) fn shown(cx: &App) -> Option<String> {
    let state = cx.try_global::<GlobalHotkey>()?;
    let registered = state.hotkey.is_some() || (state.paused && state.error.is_none());
    state.applied.clone().filter(|_| registered)
}

/// Whether the hotkey is held off now, and whether the system holds it (for the tests that close
/// Settings mid-recording).
#[cfg(test)]
pub(crate) fn state_for_test(cx: &App) -> (bool, bool) {
    let state = cx.global::<GlobalHotkey>();
    (state.paused, state.hotkey.is_some())
}

impl GlobalHotkey {
    fn apply(&mut self, spec: &str) {
        // Release the old key first, or re-registering the same combination fails.
        self.hotkey = None;
        self.error = None;
        self.applied = Some(spec.to_string());
        if !registers(spec, self.paused) {
            return;
        }
        match Hotkey::register(spec) {
            Ok(key) => {
                let tx = self.tx.clone();
                Hotkey::on_press(move || {
                    let _ = tx.try_send(MenuCommand::Hotkey);
                });
                self.hotkey = Some(key);
            }
            Err(err) => {
                tracing::warn!("global hotkey: {err}");
                self.error = Some(err);
            }
        }
    }
}

/// Whether a hotkey spec is to be registered now: never an empty one, and not while the
/// recorder in Settings has the keys.
fn registers(spec: &str, paused: bool) -> bool {
    !spec.is_empty() && !paused
}

#[cfg(test)]
mod tests {
    use super::registers;

    /// While the settings window records a new hotkey the current one is held off, so
    /// pressing it again records it instead of opening the quick panel.
    #[test]
    fn a_recording_holds_the_hotkey_off() {
        assert!(registers("alt-space", false));
        assert!(!registers("alt-space", true));
        assert!(!registers("", false), "no hotkey set");
    }
}
