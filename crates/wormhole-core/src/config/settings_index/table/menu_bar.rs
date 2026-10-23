//! The Menu bar section: the status item and the global hotkey.

use super::{entry, group};
use crate::config::settings_index::Group;

pub(super) const MENU_BAR: &[Group] = &[
    group(
        "Menu bar",
        &[
            entry(
                "menu-bar",
                "Show Wormhole in the menu bar",
                "Terminals by status and quick actions",
                "status bar tray item menubar",
                &["menu_bar"],
            ),
            entry(
                "menu-bar-icon",
                "Icon shows",
                "The dot and count on the status item",
                "status item icon count",
                &["menu_bar_icon"],
            ),
            entry(
                "menu-bar-animate",
                "Animate menu bar icon while working",
                "The prompt in the icon types itself out while something runs. Off: a still, filled icon",
                "animation",
                &["menu_bar_animate"],
            ),
        ],
    ),
    group(
        "Global hotkey",
        &[
            entry(
                "hotkey",
                "Shortcut",
                "Works from any app. Click, then press the keys",
                "global shortcut quake record hotkey",
                &["hotkey"],
            ),
            entry(
                "hotkey-action",
                "Hotkey opens",
                "Pressing it again with the panel open shows Wormhole",
                "quick panel toggle",
                &["hotkey_action"],
            ),
            entry(
                "jump-waiting-key",
                "Jump to next waiting terminal",
                "Inside Wormhole, in any window",
                "needs you waiting shortcut",
                &["jump_waiting_key"],
            ),
        ],
    ),
];
