//! The shortcut recorder of a settings row: click the field, press the keys. Esc
//! stops without a change, Delete (or the ×) clears it, keys the row can't use wait for others
//! (a key without a modifier, one the hotkey can't register), and keys that macOS, the app or the
//! shell already answer to are shown in the warning state with who has them ("The shell uses
//! ⌥Space"), instead of being taken. A shortcut already set that one of them has, or that failed
//! to register, shows the same state.
//!
//! Who has a key already is the key table's answer (`keymap::uses`, through
//! `shortcut_record::conflict`): the app's actions, macOS's own keys, and ⌥ or ⌃ without ⌘,
//! which the shell needs for Meta, word jumps and control characters.
//!
//! While the global hotkey's own field records, the hotkey is held off, so pressing the
//! current keys records them instead of opening the quick panel.

use wormhole_core::keymap::ActionId;
use wormhole_core::keyspec::{KeyMods, KeySpec};
use wormhole_core::shortcut_record::{self, Recorded};

use super::*;
use style::RecorderState;

/// A recorder in progress, or one that refused keys.
pub(super) enum Recording {
    /// Waiting for keys for the row `id`; the modifiers held so far.
    Keys { id: &'static str, held: KeyMods },
    /// The keys pressed for `id` are taken (`conflict` says by what): shown, not taken.
    Taken { id: &'static str, spec: KeySpec, conflict: String },
}

/// The keys of a spec as key caps: the modifiers in ⌃⌥⇧⌘ order, then the key.
fn caps(mods: KeyMods, key: Option<&str>) -> Vec<SharedString> {
    let mut caps: Vec<SharedString> = [(mods.ctrl, "⌃"), (mods.alt, "⌥"), (mods.shift, "⇧"), (mods.cmd, "⌘")]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, glyph)| glyph.into())
        .collect();
    if let Some(key) = key {
        let spec = KeySpec { mods: KeyMods::default(), key: key.to_string() };
        caps.push(spec.glyphs().into());
    }
    caps
}

fn mods_of(modifiers: &Modifiers) -> KeyMods {
    KeyMods { cmd: modifiers.platform, ctrl: modifiers.control, alt: modifiers.alt, shift: modifiers.shift }
}

/// The row whose recorder sets the global hotkey.
const HOTKEY_ROW: &str = "hotkey";

/// Whether the global hotkey is to be held off: only while its own field waits for keys. A
/// field showing keys it refused isn't recording any more.
fn pauses_hotkey(recording: Option<&Recording>) -> bool {
    matches!(recording, Some(Recording::Keys { id, .. }) if *id == HOTKEY_ROW)
}

/// What a recorder row records into and checks.
#[derive(Clone, Copy)]
pub(super) struct RecorderRow {
    pub get: fn(&Settings) -> &str,
    pub set: fn(&mut Settings, String),
    /// Whether the row can use these keys (a modifier, a key the system can register).
    pub accepts: fn(&KeySpec) -> bool,
    /// What is wrong with the shortcut set now (macOS has it, it didn't register).
    pub problem: fn(&Settings, &App) -> Option<String>,
    /// The action whose key this is (its own key is no conflict).
    pub own: Option<ActionId>,
}

impl SettingsWindow {
    /// Start, change or end a recording. The global hotkey is held off while its own field
    /// records and put back as soon as that ends, however it ends.
    pub(super) fn set_recording(&mut self, recording: Option<Recording>, cx: &mut App) {
        let paused = pauses_hotkey(recording.as_ref());
        self.recording = recording;
        app_mut(cx).menu.pause_hotkey(paused);
    }

    /// Esc: a recording ends without a change; whether there was one.
    pub(super) fn end_recording(&mut self, cx: &mut App) -> bool {
        if self.recording.is_none() {
            return false;
        }
        self.set_recording(None, cx);
        true
    }

    /// The recorder of row `id`.
    pub(super) fn render_recorder(
        &self,
        id: &'static str,
        row: RecorderRow,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let ui = ui(cx);
        let current = (row.get)(settings(cx)).trim().to_string();
        let parsed = current.parse::<KeySpec>().ok();
        let problem = parsed.as_ref().and_then(|_| (row.problem)(settings(cx), cx));
        let (state, warning) = match &self.recording {
            Some(Recording::Keys { id: recording, held }) if *recording == id => {
                (RecorderState::Recording(caps(*held, None)), None)
            }
            Some(Recording::Taken { id: taken, spec, conflict }) if *taken == id => (
                RecorderState::Conflict(caps(spec.mods, Some(&spec.key))),
                Some(format!("{conflict}. Press other keys.")),
            ),
            _ => match parsed {
                Some(spec) if problem.is_some() => {
                    (RecorderState::Conflict(caps(spec.mods, Some(&spec.key))), problem)
                }
                Some(spec) => (RecorderState::Set(caps(spec.mods, Some(&spec.key))), None),
                None => (RecorderState::Empty, None),
            },
        };
        let set = row.set;
        let on_clear: style::ClickHandler =
            Box::new(move |_, _, cx| update_settings(cx, |s| set(s, String::new())));
        div()
            .flex()
            .flex_col()
            .items_end()
            .gap_1()
            .child(
                style::shortcut_recorder(
                    SharedString::from(format!("recorder-{id}")),
                    ui,
                    state,
                    Some(on_clear),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.set_recording(Some(Recording::Keys { id, held: KeyMods::default() }), cx);
                    // Keys go to the window, which hands them to the recorder.
                    window.focus(&this.focus_handle, cx);
                    cx.notify();
                })),
            )
            .children(warning.map(|warning| div().text_caption().text_color(ui.warning_text).child(warning)))
            .into_any_element()
    }

    /// A key press while a recorder waits: whether it was the recorder's.
    pub(super) fn record_key(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) -> bool {
        let Some(Recording::Keys { id, .. }) = self.recording else { return false };
        let Some(row) = registry::recorder_row(id) else { return false };
        match shortcut_record::record(mods_of(&keystroke.modifiers), &keystroke.key) {
            Recorded::Cancel => self.set_recording(None, cx),
            Recorded::Clear => {
                self.set_recording(None, cx);
                update_settings(cx, |s| (row.set)(s, String::new()));
            }
            Recorded::NeedsModifier => {}
            // Keys this row can't use: wait for others.
            Recorded::Shortcut(spec) if !(row.accepts)(&spec) => {}
            Recorded::Shortcut(spec) => match shortcut_record::conflict(&spec, row.own) {
                Some(conflict) => self.set_recording(Some(Recording::Taken { id, spec, conflict }), cx),
                None => {
                    self.set_recording(None, cx);
                    update_settings(cx, |s| (row.set)(s, spec.to_string()));
                }
            },
        }
        cx.notify();
        true
    }

    /// The modifiers held while a recorder waits, shown in the field.
    pub(super) fn record_modifiers(&mut self, modifiers: &Modifiers, cx: &mut Context<Self>) {
        if let Some(Recording::Keys { held, .. }) = &mut self.recording {
            *held = mods_of(modifiers);
            cx.notify();
        }
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::keyspec::{KeyMods, KeySpec};

    use super::{Recording, caps, pauses_hotkey};

    #[test]
    fn caps_read_in_apples_order() {
        let mods = KeyMods { cmd: true, ctrl: false, alt: true, shift: true };
        let caps: Vec<String> = caps(mods, Some("space")).into_iter().map(|c| c.to_string()).collect();
        assert_eq!(caps, ["⌥", "⇧", "⌘", "Space"]);
    }

    /// The hotkey is held off only while its own field waits for keys: not for the jump
    /// key's field, and not once keys were refused (the field isn't recording then).
    #[test]
    fn only_the_hotkeys_own_recording_pauses_it() {
        let keys = |id| Recording::Keys { id, held: KeyMods::default() };
        assert!(pauses_hotkey(Some(&keys("hotkey"))));
        assert!(!pauses_hotkey(Some(&keys("jump-waiting-key"))));
        let spec: KeySpec = "alt-space".parse().expect("spec");
        let taken = Recording::Taken { id: "hotkey", spec, conflict: "The shell uses ⌥Space".into() };
        assert!(!pauses_hotkey(Some(&taken)));
        assert!(!pauses_hotkey(None));
    }
}
