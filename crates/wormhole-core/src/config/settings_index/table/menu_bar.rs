//! The Menu bar section: the status item and the global hotkey.

use super::{literal_entry, literal_group};
use crate::config::settings_index::Group;

pub(super) const MENU_BAR: &[Group] = &[
    literal_group(
        "Menu bar",
        &[
            literal_entry(
                "menu-bar",
                "Show Wormhole in the menu bar",
                "Terminals by status and quick actions",
                "status bar tray item menubar",
                &["menu_bar"],
            ),
            literal_entry(
                "menu-bar-icon",
                "Icon shows",
                "The dot and count on the status item",
                "status item icon count",
                &["menu_bar_icon"],
            ),
            literal_entry(
                "menu-bar-animate",
                "Animate menu bar icon while working",
                "The prompt in the icon types itself out while something runs. Off: a still, filled icon",
                "animation",
                &["menu_bar_animate"],
            ),
        ],
    ),
    literal_group(
        "Global hotkey",
        &[
            literal_entry(
                "hotkey",
                "Shortcut",
                "Works from any app. Click, then press the keys",
                "global shortcut quake record hotkey",
                &["hotkey"],
            ),
            literal_entry(
                "hotkey-action",
                "Hotkey opens",
                "Pressing it again with the panel open shows Wormhole",
                "quick panel toggle",
                &["hotkey_action"],
            ),
            literal_entry(
                "jump-waiting-key",
                "Jump to next waiting terminal",
                "Inside Wormhole, in any window",
                "needs you waiting shortcut",
                &["jump_waiting_key"],
            ),
        ],
    ),
];
