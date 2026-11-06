//! Menu bar: the status item and what its icon shows.

use wormhole_core::config::MenuBarIcon;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use crate::settings_window::registry::{Segment, Setting, toggle};

pub(super) const SETTINGS: &[Setting] = &[
    toggle!("menu-bar", menu_bar),
    Setting::segmented(
        "menu-bar-icon",
        &[
            Segment::text(SETTINGS_MENU_BAR_STATUS_AND_COUNT),
            Segment::text(SETTINGS_MENU_BAR_STATUS),
            Segment::text(SETTINGS_MENU_BAR_ICON_ONLY),
        ],
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
];
