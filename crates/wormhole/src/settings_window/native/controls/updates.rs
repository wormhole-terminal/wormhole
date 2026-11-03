//! The controls of the updates page the native window writes through: each setting's menu or switch over the settings.

use wormhole_core::update::UpdateChannel;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

use super::{MenuOption, Setting, toggle};

pub(super) const SETTINGS: &[Setting] = &[
    toggle!("auto-update", auto_update),
    Setting::menu(
        "update-channel",
        &[MenuOption::text(SETTINGS_UPDATES_STABLE), MenuOption::text(SETTINGS_UPDATES_UNSTABLE)],
        |s| usize::from(s.update_channel == UpdateChannel::Unstable),
        |s, ix| s.update_channel = if ix == 1 { UpdateChannel::Unstable } else { UpdateChannel::Stable },
    ),
    toggle!("anonymous-count", anonymous_count),
];
