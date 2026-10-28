//! Opening and closing a warning's explanation, the popover under its ⓘ (`info` says what is in it). The
//! ⓘ, and a click on the capsule, open it; a click on the ⓘ while it is open closes it; the chip going
//! away (the × or the terminal's end) closes it too.

use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::view::notice::Spot;

use super::chips::Chips;
use crate::platform::notice::Chip;
use crate::settings::store_in;

/// A click on the ⓘ that began about when the explanation closed is the click that closed it (a popover goes
/// away on the mouse's press outside it, and the release reaches the button after): it does not open it
/// again. Told by when the press began, so a long press does not open it on release.
const SAME_CLICK: Duration = Duration::from_millis(250);

/// Open the explanation of `window`'s warning, or close it when it is open.
pub(super) fn toggle(window: &mut Window, down: Instant, cx: &mut App) {
    let id = window.window_handle().window_id();
    let Some(chips) = cx.try_global::<Chips>() else { return };
    let Some(entry) = chips.windows.get(&id) else { return };
    if let Some((_, open)) = entry.info.clone() {
        open.close(cx);
        return;
    }
    if entry.info_closed.is_some_and(|at| at.max(down) - at.min(down) < SAME_CLICK) {
        return;
    }
    let Some(notice) = entry.rules.held() else { return };
    let Some(profile) = chips.homes.get(&id).and_then(|home| home(cx)).map(|facts| facts.profile) else {
        return;
    };
    let appearance = store_in(&profile, cx).tokens.appearance;
    let logs = wormhole_core::storage::paths::DataRoot::current().logs_dir();
    let Some(popover) = super::info::popover(notice, entry.sticky_detail.as_deref(), logs, appearance) else {
        return;
    };
    let anchor = entry.chip.as_ref().and_then(Chip::info_bounds).unwrap_or_else(|| fallback(entry.spot));
    let which = entry.opened + 1;
    let popover = popover.on_close(move |_, cx| closed(id, which, cx));
    // Outside the global's update: opening makes AppKit call GPUI back (see `platform::popover`).
    let handle = crate::platform::show_popover(popover, anchor, window, cx);
    if let Some(entry) = cx.global_mut::<Chips>().windows.get_mut(&id) {
        entry.opened = which;
        entry.info = Some((which, handle));
    }
}

/// The explanation `which` is gone, by itself or closed from here.
fn closed(id: WindowId, which: u64, cx: &mut App) {
    let now = cx.background_executor().now();
    if cx.try_global::<Chips>().is_none() {
        return;
    }
    if let Some(entry) = cx.global_mut::<Chips>().windows.get_mut(&id)
        && entry.info.as_ref().is_some_and(|(open, _)| *open == which)
    {
        entry.info = None;
        entry.info_closed = Some(now);
    }
}

/// Where the arrow points when there is no panel to ask (a window that can't have one): the top middle of
/// the terminal the chip is about.
fn fallback(spot: Option<Spot>) -> Bounds<Pixels> {
    let anchor = spot.map(|spot| spot.anchor);
    let (x, y) = anchor.map_or((0.0, 0.0), |a| (a.x + a.w / 2.0, a.y + 8.0));
    Bounds::new(point(px(x), px(y)), size(px(20.0), px(20.0)))
}
