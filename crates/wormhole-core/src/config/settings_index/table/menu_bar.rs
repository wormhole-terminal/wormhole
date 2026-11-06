//! The Menu bar section: the status item and what its icon shows.

use super::{entry, untitled};
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const MENU_BAR: &[Group] = &[untitled(&[
    entry("menu-bar", SETTINGS_MENU_BAR_MENU_BAR, &["menu_bar"])
        .with_description(SETTINGS_MENU_BAR_MENU_BAR_DESCRIPTION)
        .with_keywords(SETTINGS_MENU_BAR_MENU_BAR_KEYWORDS),
    entry("menu-bar-icon", SETTINGS_MENU_BAR_ICON, &["menu_bar_icon"])
        .with_description(SETTINGS_MENU_BAR_ICON_DESCRIPTION)
        .with_keywords(SETTINGS_MENU_BAR_ICON_KEYWORDS),
    entry("menu-bar-animate", SETTINGS_MENU_BAR_ANIMATE, &["menu_bar_animate"])
        .with_description(SETTINGS_MENU_BAR_ANIMATE_DESCRIPTION)
        .with_keywords(SETTINGS_MENU_BAR_ANIMATE_KEYWORDS),
])];
