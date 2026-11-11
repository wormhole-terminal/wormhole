//! A recorded shortcut checked by core's key rules, so the recorder row says what the GPUI one says:
//! what a key press does to the field (`shortcut_record::record`), whether the keys are free
//! (`shortcut_record::conflict`: the app's actions, macOS's own keys, the shell's) and the spelling
//! `config.json` stores (`KeySpec`'s Display). Swift keeps what is the keyboard's: turning a key
//! event into the spec's text.

use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keyspec::KeySpec;
use wormhole_core::keys::shortcut_record::{Recorded, conflict, record};

/// The setting whose shortcut works inside the app only, and so needs a modifier besides Shift.
const JUMP_WAITING_KEY: &str = "jump_waiting_key";

/// What the field does with a key press.
#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum ShortcutVerdict {
    /// Take it: `spec` is what to store, `glyphs` how to show it (`⇧⌘R`).
    Accepted { spec: String, glyphs: String },
    /// Esc alone: stop recording and keep the shortcut there was.
    Cancel,
    /// Delete alone: no shortcut.
    Clear,
    /// The key alone would swallow typing: keep waiting.
    NeedsModifier,
    /// Not taken, and why, in the field's words (`The shell uses ⌥Space`).
    Refused { glyphs: String, reason: String },
    /// The text is no shortcut (`config.json` spelling).
    Invalid { reason: String },
}

/// Checks the shortcut a key press recorded for the setting `config_key` (`jump_waiting_key` works
/// inside the app only; any other, `hotkey` among them, is checked as a global shortcut). `spec` is
/// `config.json`'s spelling (`ctrl-alt-shift-cmd-key`).
#[uniffi::export]
pub fn check_shortcut(config_key: String, spec: String) -> ShortcutVerdict {
    let parsed = match spec.parse::<KeySpec>() {
        Ok(parsed) => parsed,
        Err(err) => return ShortcutVerdict::Invalid { reason: err.to_string() },
    };
    let shortcut = match record(parsed.mods, &parsed.key) {
        Recorded::Cancel => return ShortcutVerdict::Cancel,
        Recorded::Clear => return ShortcutVerdict::Clear,
        Recorded::NeedsModifier => return ShortcutVerdict::NeedsModifier,
        Recorded::Shortcut(shortcut) => shortcut,
    };
    let glyphs = shortcut.glyphs();
    // The jump key works inside the app: a modifier that isn't Shift alone (that types a capital).
    let jump_needs_modifier =
        config_key == JUMP_WAITING_KEY && !(shortcut.mods.cmd || shortcut.mods.ctrl || shortcut.mods.alt);
    if !shortcut.has_known_key() || jump_needs_modifier {
        return ShortcutVerdict::NeedsModifier;
    }
    let own = (config_key == JUMP_WAITING_KEY).then_some(ActionId::GoToWaiting);
    match conflict(&shortcut, own) {
        Some(reason) => ShortcutVerdict::Refused { glyphs, reason },
        None => ShortcutVerdict::Accepted { spec: shortcut.to_string(), glyphs },
    }
}

#[cfg(test)]
mod tests {
    use super::{JUMP_WAITING_KEY, ShortcutVerdict, check_shortcut};

    const HOTKEY: &str = "hotkey";

    #[test]
    fn a_free_shortcut_is_taken_as_stored() {
        match check_shortcut(HOTKEY.into(), "shift-cmd-space".into()) {
            ShortcutVerdict::Accepted { spec, glyphs } => {
                assert_eq!(spec, "shift-cmd-space");
                assert_eq!(glyphs, "⇧⌘Space");
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(check_shortcut(HOTKEY.into(), "f5".into()), ShortcutVerdict::Accepted { .. }));
    }

    #[test]
    fn keys_that_are_taken_say_by_whom() {
        match check_shortcut(HOTKEY.into(), "cmd-space".into()) {
            ShortcutVerdict::Refused { reason, .. } => assert_eq!(reason, "macOS uses ⌘Space for Spotlight"),
            other => panic!("{other:?}"),
        }
        match check_shortcut(HOTKEY.into(), "alt-space".into()) {
            ShortcutVerdict::Refused { reason, .. } => assert_eq!(reason, "The shell uses ⌥Space"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_jump_keys_own_action_is_no_conflict_for_it_alone() {
        assert!(matches!(
            check_shortcut(JUMP_WAITING_KEY.into(), "alt-cmd-j".into()),
            ShortcutVerdict::Accepted { .. }
        ));
        assert!(matches!(check_shortcut(HOTKEY.into(), "alt-cmd-j".into()), ShortcutVerdict::Refused { .. }));
    }

    #[test]
    fn esc_delete_and_a_bare_key_are_the_fields_not_shortcuts() {
        let verdict = |key: &str, spec: &str| check_shortcut(key.into(), spec.into());
        assert_eq!(verdict(HOTKEY, "escape"), ShortcutVerdict::Cancel);
        assert_eq!(verdict(HOTKEY, "backspace"), ShortcutVerdict::Clear);
        assert_eq!(verdict(HOTKEY, "a"), ShortcutVerdict::NeedsModifier);
        assert_eq!(verdict(JUMP_WAITING_KEY, "shift-j"), ShortcutVerdict::NeedsModifier);
        assert!(matches!(verdict(HOTKEY, "cmd-"), ShortcutVerdict::Invalid { .. }));
    }
}
