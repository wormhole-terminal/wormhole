//! The Menu bar section: the status item and the global hotkey.

use super::{entry, group};
use crate::config::settings_index::Group;
use wormhole_text::keys::*;

pub(super) const MENU_BAR: &[Group] = &[
    group(
        SETTINGS_MENU_BAR_GROUP_MENU_BAR,
        &[
            entry("menu-bar", SETTINGS_MENU_BAR_MENU_BAR, &["menu_bar"])
                .with_description(SETTINGS_MENU_BAR_MENU_BAR_DESCRIPTION)
                .with_keywords(SETTINGS_MENU_BAR_MENU_BAR_KEYWORDS),
            entry("menu-bar-icon", SETTINGS_MENU_BAR_ICON, &["menu_bar_icon"])
                .with_description(SETTINGS_MENU_BAR_ICON_DESCRIPTION)
                .with_keywords(SETTINGS_MENU_BAR_ICON_KEYWORDS),
            entry("menu-bar-animate", SETTINGS_MENU_BAR_ANIMATE, &["menu_bar_animate"])
                .with_description(SETTINGS_MENU_BAR_ANIMATE_DESCRIPTION)
                .with_keywords(SETTINGS_MENU_BAR_ANIMATE_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_MENU_BAR_GROUP_GLOBAL_HOTKEY,
        &[
            entry("hotkey", SETTINGS_MENU_BAR_HOTKEY, &["hotkey"])
                .with_description(SETTINGS_MENU_BAR_HOTKEY_DESCRIPTION)
                .with_keywords(SETTINGS_MENU_BAR_HOTKEY_KEYWORDS),
            entry("hotkey-action", SETTINGS_MENU_BAR_HOTKEY_ACTION, &["hotkey_action"])
                .with_description(SETTINGS_MENU_BAR_HOTKEY_ACTION_DESCRIPTION)
                .with_keywords(SETTINGS_MENU_BAR_HOTKEY_ACTION_KEYWORDS),
            entry("jump-waiting-key", SETTINGS_MENU_BAR_JUMP_WAITING_KEY, &["jump_waiting_key"])
                .with_description(SETTINGS_MENU_BAR_JUMP_WAITING_KEY_DESCRIPTION)
                .with_keywords(SETTINGS_MENU_BAR_JUMP_WAITING_KEY_KEYWORDS),
        ],
    ),
];
