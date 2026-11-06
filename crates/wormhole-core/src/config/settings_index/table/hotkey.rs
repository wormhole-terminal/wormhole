//! The Hotkey section: the global shortcut, what it opens, and the key that jumps to the waiting terminal.

use super::{entry, untitled};
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const HOTKEY: &[Group] = &[untitled(&[
    entry("hotkey", SETTINGS_HOTKEY_HOTKEY, &["hotkey"])
        .with_description(SETTINGS_HOTKEY_HOTKEY_DESCRIPTION)
        .with_keywords(SETTINGS_HOTKEY_HOTKEY_KEYWORDS),
    entry("hotkey-action", SETTINGS_HOTKEY_ACTION, &["hotkey_action"])
        .with_description(SETTINGS_HOTKEY_ACTION_DESCRIPTION)
        .with_keywords(SETTINGS_HOTKEY_ACTION_KEYWORDS),
    entry("jump-waiting-key", SETTINGS_HOTKEY_JUMP_WAITING_KEY, &["jump_waiting_key"])
        .with_description(SETTINGS_HOTKEY_JUMP_WAITING_KEY_DESCRIPTION)
        .with_keywords(SETTINGS_HOTKEY_JUMP_WAITING_KEY_KEYWORDS),
])];
