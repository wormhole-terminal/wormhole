//! Menu bar & hotkey: the status item and what its icon shows, the global hotkey (recorded,
//! not typed) and what it opens, and the key that jumps to the terminal waiting longest.

use gpui_kit::*;
use wormhole_core::config::{HotkeyAction, MenuBarIcon};
use wormhole_core::keys::keymap::ActionId;
use wormhole_core::keys::keyspec::KeySpec;

use crate::settings::Settings;
use crate::settings_window::recorder::RecorderRow;
use crate::settings_window::registry::{Segment, Setting, toggle};
use crate::workspace::app::app;

pub(super) const SETTINGS: &[Setting] = &[
    toggle!("menu-bar", menu_bar),
    Setting::segmented(
        "menu-bar-icon",
        &[Segment::text("Status and count"), Segment::text("Status"), Segment::text("Icon only")],
        |s| match s.menu_bar_icon {
            MenuBarIcon::StatusCount => 0,
            MenuBarIcon::Status => 1,
            MenuBarIcon::Plain => 2,
        },
        |s, ix| {
            s.menu_bar_icon = [MenuBarIcon::StatusCount, MenuBarIcon::Status, MenuBarIcon::Plain][ix.min(2)]
        },
    ),
    toggle!("menu-bar-animate", menu_bar_animate),
    Setting::recorder(
        "hotkey",
        RecorderRow {
            get: |s| &s.hotkey,
            set: |s, keys| s.hotkey = keys,
            accepts: hotkey_accepts,
            problem: hotkey_problem,
            own: None,
        },
    ),
    Setting::segmented(
        "hotkey-action",
        &[Segment::text("Quick panel"), Segment::text("Show or hide Wormhole")],
        |s| usize::from(s.hotkey_action == HotkeyAction::ToggleApp),
        |s, ix| s.hotkey_action = if ix == 1 { HotkeyAction::ToggleApp } else { HotkeyAction::QuickPanel },
    ),
    Setting::recorder(
        "jump-waiting-key",
        RecorderRow {
            get: |s| &s.jump_waiting_key,
            set: |s, keys| s.jump_waiting_key = keys,
            accepts: jump_accepts,
            problem: |_, _| None,
            own: Some(ActionId::GoToWaiting),
        },
    ),
];

/// A global hotkey needs a modifier (or an F-key) and a key the system can register.
fn hotkey_accepts(spec: &KeySpec) -> bool {
    crate::platform::hotkey::parse_spec(&spec.to_string()).is_ok()
}

/// The jump key works inside the app: a modifier that isn't Shift alone (that types a capital).
fn jump_accepts(spec: &KeySpec) -> bool {
    spec.has_known_key() && (spec.mods.cmd || spec.mods.ctrl || spec.mods.alt)
}

/// Why the hotkey set now doesn't work: macOS, the app or the shell has the keys (set by hand in
/// config.json; the recorder refuses them), or it didn't register.
fn hotkey_problem(s: &Settings, cx: &App) -> Option<String> {
    crate::platform::hotkey::system_conflict(&s.hotkey).or_else(|| app(cx).menu.hotkey_error.clone())
}

#[cfg(test)]
mod tests {
    use wormhole_core::keys::keyspec::KeySpec;

    use super::{hotkey_accepts, jump_accepts};

    fn spec(text: &str) -> KeySpec {
        text.parse().expect("valid spec")
    }

    #[test]
    fn a_shortcut_needs_a_modifier_and_a_key_that_exists() {
        assert!(hotkey_accepts(&spec("shift-cmd-space")));
        assert!(hotkey_accepts(&spec("f12")), "an F-key alone is fine globally");
        assert!(!hotkey_accepts(&spec("j")), "a bare key would eat typing");
        assert!(jump_accepts(&spec("alt-cmd-j")));
        assert!(!jump_accepts(&spec("shift-j")), "shift alone types a capital");
        assert_eq!(spec("alt-cmd-j").to_string(), "alt-cmd-j", "recorded as the config spells it");
    }
}
