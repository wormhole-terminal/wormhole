//! A warning that stays has to stay with its terminal. The terminal moves without telling the chip (a
//! terminal selected, the sidebar folded, a split opened, the panel shown), so while a sticky notice is held
//! a small task looks again every [`EVERY`]: it puts the chip back at the terminal's top edge, orders the
//! panel out while the terminal is not on screen or something is drawn over the window (a chip taking
//! clicks over a palette would take them from it), takes the warning away once its terminal is closed, and
//! makes the panel again when the theme's look or the system's transparency changed. Only a held warning
//! has the task, so a window with none does no work.

use std::time::Duration;

use gpui_kit::*;
use wormhole_core::view::motion::notice::Entrance;

use super::chips::{Chips, Facts, Where, close_info, content, spot_for, withdraw};
use crate::platform::notice::Chip;
use crate::settings::store_in;

/// How often a held warning looks at where its terminal is: short enough that a chip does not stay over the
/// wrong terminal for long, and nothing but reads between two looks.
pub(crate) const EVERY: Duration = Duration::from_millis(100);

/// The task that keeps `window`'s sticky chip at its terminal, until the warning is gone or it is dropped.
pub(super) fn watch(window: AnyWindowHandle, cx: &mut App) -> Task<()> {
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(EVERY).await;
            let more = cx.update(|cx| window.update(cx, |_, window, cx| step(window, cx)).unwrap_or(false));
            if !more {
                break;
            }
        }
    })
}

/// Whether a warning about the terminal `placed` says (`None`: about no terminal in particular) waits out of
/// sight in a window that tells `facts`.
pub(super) fn hidden(facts: &Facts, placed: Option<&Where>) -> bool {
    let off_screen = match placed {
        Some(Where::At { terminal, .. }) => !facts.shown.contains(terminal),
        Some(Where::Gone) | None => false,
    };
    facts.covered || off_screen
}

/// One look. `false` when there is nothing more to keep an eye on.
fn step(window: &mut Window, cx: &mut App) -> bool {
    let id = window.window_handle().window_id();
    let Some(chips) = cx.try_global::<Chips>() else { return false };
    let Some(entry) = chips.windows.get(&id) else { return false };
    if entry.rules.held().is_none() {
        return false;
    }
    let (anchor, clickable) = (entry.sticky_anchor.clone(), entry.rules.clickable());
    let Some(facts) = chips.homes.get(&id).and_then(|home| home(cx)) else { return true };
    let placed = anchor.as_ref().map(|anchor| anchor(cx));
    if matches!(placed, Some(Where::Gone)) {
        // The terminal it was about is closed: nothing to warn about any more.
        withdraw(id, cx);
        return false;
    }
    let bounds = match &placed {
        Some(Where::At { bounds, .. }) => Some(*bounds),
        _ => facts.selected,
    };
    let (spot, away) = (spot_for(window, bounds), hidden(&facts, placed.as_ref()));
    let store = store_in(&facts.profile, cx);
    let look = (store.tokens.appearance, store.system().opaque());

    cx.update_global::<Chips, _>(|chips, cx| {
        let Some(entry) = chips.windows.get_mut(&id) else { return };
        if let Some((theme, old_opaque)) = entry.look
            && (theme, old_opaque) != look
        {
            entry.look = Some(look);
            entry.theme = Some(look.0);
            let (words, glyph) = (entry.words.clone(), entry.glyph);
            let buttons = entry.rules.buttons();
            let content = glyph.map(|glyph| content(glyph, &words, look.0, buttons));
            if look.1 != old_opaque {
                // Another material is another view: the panel is made again, and fades in at once.
                close_info(entry, cx);
                if let Some(chip) = entry.chip.take() {
                    chip.close(cx);
                }
                entry.hidden = false;
                if let (Some(clicks), Some(content)) = (entry.clicks.clone(), &content) {
                    entry.chip = Chip::new(window, look.1, clicks);
                    if let Some(chip) = &mut entry.chip {
                        chip.show(content, spot, Entrance::Fade, entry.rules.clickable(), cx);
                    }
                }
            } else if let (Some(chip), Some(content)) = (&mut entry.chip, &content) {
                chip.restyle(content);
            }
        }
        // A copy's words keep the place they came with; the warning is what follows its terminal.
        // The explanation's arrow points at the chip where it is: out of sight or moved, it closes.
        if clickable && (away || entry.spot != Some(spot)) {
            close_info(entry, cx);
        }
        if clickable {
            if let Some(chip) = &mut entry.chip {
                if away != entry.hidden {
                    chip.set_hidden(away, cx);
                }
                if !away && entry.spot != Some(spot) {
                    chip.place(spot);
                }
            }
            entry.hidden = away;
            if !away {
                entry.spot = Some(spot);
            }
        }
    });
    true
}
