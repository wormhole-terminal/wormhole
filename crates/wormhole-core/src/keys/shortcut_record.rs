//! Recording a shortcut (the global hotkey, the jump to the waiting terminal):
//! what a key press does to the field, and whether the keys are already taken (by the app, macOS
//! or the shell), so the field can say so instead of taking them. Who has a key is the key
//! table's answer (`keymap::uses`).

use crate::keys::keymap::{self, ActionId, KeyUse};
use crate::keys::keyspec::{KeyMods, KeySpec};

/// What a key press while recording does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Recorded {
    /// Esc alone: stop recording, keep the shortcut there was.
    Cancel,
    /// Delete or Backspace alone: no shortcut.
    Clear,
    /// A key without a modifier (other than an F-key) would swallow normal typing: keep
    /// waiting.
    NeedsModifier,
    Shortcut(KeySpec),
}

/// The key `key` (GPUI's name, lowercase) pressed with `mods`. Modifier keys on their own
/// never get here (they are shown as held, not recorded).
pub fn record(mods: KeyMods, key: &str) -> Recorded {
    let bare = mods == KeyMods::default();
    match key {
        "escape" if bare => Recorded::Cancel,
        "backspace" | "delete" if bare => Recorded::Clear,
        _ => {
            let spec = KeySpec { mods, key: key.to_lowercase() };
            let f_key = spec
                .key
                .strip_prefix('f')
                .and_then(|n| n.parse::<u8>().ok())
                .is_some_and(|n| (1..=20).contains(&n));
            if bare && !f_key { Recorded::NeedsModifier } else { Recorded::Shortcut(spec) }
        }
    }
}

/// What already has `spec`, as the field says it ("⌘T is New Terminal", "macOS uses ⌘Space for
/// Spotlight", "The shell uses ⌥Space"), or nothing. `own` is the action the field sets the key
/// of (the jump key is Go to Waiting Terminal's), which doesn't count. Keys the shell needs (⌥ or
/// ⌃ without ⌘: Meta, word jumps, control characters) are taken too: as a global hotkey ⌥Space
/// would never reach a terminal again, which is rarely what the person meant.
pub fn conflict(spec: &KeySpec, own: Option<ActionId>) -> Option<String> {
    match keymap::uses(spec)? {
        KeyUse::Action(entry, _) if Some(entry.id) == own => None,
        used @ KeyUse::Action(..) => Some(used.to_string()),
        KeyUse::System(what) => Some(format!("macOS uses {} for {what}", spec.glyphs())),
        KeyUse::Shell(_) => Some(format!("The shell uses {}", spec.glyphs())),
    }
}

#[cfg(test)]
mod tests {
    use super::{Recorded, conflict, record};
    use crate::keys::keyspec::{KeyMods, KeySpec};

    fn mods(cmd: bool, ctrl: bool, alt: bool, shift: bool) -> KeyMods {
        KeyMods { cmd, ctrl, alt, shift }
    }

    #[test]
    fn esc_cancels_delete_clears_and_a_bare_key_waits_for_a_modifier() {
        let none = KeyMods::default();
        assert_eq!(record(none, "escape"), Recorded::Cancel);
        assert_eq!(record(none, "backspace"), Recorded::Clear);
        assert_eq!(record(none, "delete"), Recorded::Clear);
        assert_eq!(record(none, "a"), Recorded::NeedsModifier);
        assert_eq!(record(none, "space"), Recorded::NeedsModifier);
        // An F-key is a shortcut on its own.
        assert_eq!(record(none, "f5"), Recorded::Shortcut("f5".parse().expect("spec")));
        // With a modifier, Esc and Delete are keys like any other.
        assert_eq!(
            record(mods(true, false, true, false), "escape"),
            Recorded::Shortcut("alt-cmd-escape".parse().expect("spec"))
        );
    }

    #[test]
    fn a_recorded_shortcut_is_what_the_hotkey_setting_reads() {
        let Recorded::Shortcut(spec) = record(mods(true, false, false, true), "Space") else {
            panic!("a shortcut");
        };
        assert_eq!(spec.to_string(), "shift-cmd-space");
        assert_eq!(spec.to_string().parse::<KeySpec>().expect("reads back"), spec);
    }

    #[test]
    fn the_app_macos_and_shell_keys_are_taken() {
        use crate::keys::keymap::ActionId;
        let spec = |s: &str| s.parse::<KeySpec>().expect("spec");
        assert_eq!(conflict(&spec("cmd-space"), None).as_deref(), Some("macOS uses ⌘Space for Spotlight"));
        assert_eq!(
            conflict(&spec("ctrl-space"), None).as_deref(),
            Some("macOS uses ⌃Space for switching input sources")
        );
        assert!(conflict(&spec("cmd-t"), None).is_some_and(|c| c.starts_with("⌘T is ")), "an app action");
        assert_eq!(conflict(&spec("shift-cmd-space"), None), None, "the suggested shortcut is free");
        // Keys the shell needs are refused, with why: ⌥ is Meta and word jumps, ⌃ a
        // control character, ⇧ alone a capital.
        assert_eq!(conflict(&spec("alt-space"), None).as_deref(), Some("The shell uses ⌥Space"));
        assert_eq!(conflict(&spec("ctrl-r"), None).as_deref(), Some("The shell uses ⌃R"));
        assert_eq!(conflict(&spec("ctrl-alt-t"), None).as_deref(), Some("The shell uses ⌃⌥T"));
        assert_eq!(conflict(&spec("shift-a"), None).as_deref(), Some("The shell uses ⇧A"));
        // With ⌘ the shell never sees the key; an F-key is nobody's.
        assert_eq!(conflict(&spec("alt-cmd-k"), None), None);
        assert_eq!(conflict(&spec("alt-f5"), None), None);
        // The jump key's own action isn't a conflict for the jump key.
        assert!(conflict(&spec("alt-cmd-j"), None).is_some());
        assert_eq!(conflict(&spec("alt-cmd-j"), Some(ActionId::GoToWaiting)), None);
    }
}
