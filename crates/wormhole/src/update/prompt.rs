//! Saying that an update is ready, where the app already says things: the notice chip. The chip is
//! sticky, so it waits for the person; a click on it restarts (`restart::request_restart`, which
//! asks first only when a restart would end something), and its × closes it for good. Closed once,
//! it stays quiet until the next version is installed: each install has a number of its own
//! (`Warning::UpdateReady`), and the chip remembers what was closed by that number.

use gpui_kit::*;
use wormhole_core::view::notice::{Notice, Warning};
use wormhole_text::keys;

use crate::app_state::AppState;
use crate::notice;
use crate::update::{UpdatePhase, Updates, restart};

/// The install of `version` is the next one, unless it is the one already told: the number that
/// says which chip the person has seen.
pub(super) fn serial_for(told: Option<(&str, u32)>, version: &str) -> u32 {
    match told {
        Some((last, serial)) if last == version => serial,
        Some((_, serial)) => serial.wrapping_add(1),
        None => 1,
    }
}

/// An update was installed (or found on disk): offer the restart in the window in front.
pub(crate) fn announce(cx: &mut App) {
    cx.global_mut::<Updates>().offered_in = None;
    keep_offered(cx);
}

/// From the app's tick, and when something installs: while an update waits and its offer was not closed,
/// the window in front has it. The front window changes, a window closes, a profile switch parks the
/// window that showed it: each shows as the offer being in a window that is no longer the front one, and
/// it moves. A closed offer is closed for the version (the whole app's, in `Updates`), so a failed restart
/// that finds the same version installed again does not show it again; a newer version does.
pub(crate) fn keep_offered(cx: &mut App) {
    let updates = cx.global::<Updates>();
    let UpdatePhase::Installed(version) = updates.phase.clone() else { return };
    if updates.dismissed.as_deref() == Some(version.as_str()) {
        return;
    }
    let Some(front) = AppState::front_window(cx) else { return };
    if updates.offered_in == Some(front.id) {
        return;
    }
    let updates = cx.global_mut::<Updates>();
    let serial = serial_for(updates.told.as_ref().map(|(v, s)| (v.as_str(), *s)), &version);
    updates.told = Some((version.clone(), serial));
    updates.offered_in = Some(front.id);
    // The one window that has it: where it was, it goes.
    notice::withdraw_offers(cx);
    let closed_version = version.clone();
    notice::post_offer(
        front.handle,
        notice::Offer {
            notice: Notice::Warned(Warning::UpdateReady(serial)),
            words: keys::notice_warned_update_ready(&version),
            act: std::rc::Rc::new(restart::request_restart),
            closed: std::rc::Rc::new(move |cx| {
                cx.global_mut::<Updates>().dismissed = Some(closed_version.clone());
                notice::withdraw_offers(cx);
            }),
        },
        cx,
    );
}

#[cfg(test)]
mod tests {
    use super::serial_for;

    #[test]
    fn a_version_keeps_its_number_and_a_newer_one_takes_the_next() {
        assert_eq!(serial_for(None, "1.6.0"), 1);
        assert_eq!(serial_for(Some(("1.6.0", 1)), "1.6.0"), 1, "the same install, said again");
        assert_eq!(serial_for(Some(("1.6.0", 1)), "1.7.0"), 2, "a new version is not the closed chip");
    }
}
