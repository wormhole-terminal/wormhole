//! Each window's chip: core's rules, the platform's panel and the timer that moves them on, kept in a global
//! keyed by the window. Nothing here draws; a window with no native panel (a test) still runs the rules, so
//! what was said and when can be asked of it.

use std::collections::HashMap;
use std::time::Instant;

use gpui_kit::*;
use wormhole_core::view::motion::notice::Entrance;
use wormhole_core::view::notice::{Effect, Notice, Notices};

use crate::platform;
use crate::platform::notice::{Chip, Content};
use crate::settings::SettingsStore;

/// One window's chip.
#[derive(Default)]
pub(super) struct Entry {
    pub(super) rules: Notices,
    /// The panel, while there is one (from the first message until the chip is out of sight).
    pub(super) chip: Option<Chip>,
    /// What the chip says now (or said last).
    pub(super) words: String,
    /// How it came, so it goes the same way.
    entrance: Option<Entrance>,
    /// Wakes the rules when a life or a leaving is over; dropping it cancels it.
    timer: Option<Task<()>>,
    /// Puts the panel under the title row again when the window is resized or goes full screen. GPUI
    /// hands that out to an entity's context, so a small one holds it, and with it the subscription.
    watch: Option<(Entity<Watch>, Subscription)>,
}

/// What listens to the window's size for its chip.
struct Watch;

#[derive(Default)]
pub(super) struct Chips {
    pub(super) windows: HashMap<WindowId, Entry>,
}

impl Global for Chips {}

/// Say `notice` in `window`. Does nothing with the Notices flag off.
pub(crate) fn post(window: &mut Window, notice: impl Into<Notice>, cx: &mut App) {
    let notice = notice.into();
    if let Some(words) = super::words_of(notice) {
        post_words(window, notice, words, cx);
    }
}

/// Say `notice` in the window in front, for a place that has no window at hand (a menu's action, a
/// terminal's event). Later in the same turn, so it works from inside an update of that very window.
pub(crate) fn post_front(notice: impl Into<Notice>, cx: &mut App) {
    if !super::enabled(cx) {
        return;
    }
    let notice = notice.into();
    cx.defer(move |cx| {
        if let Some(front) = cx.active_window() {
            front.update(cx, |_, window, cx| post(window, notice, cx)).ok();
        }
    });
}

/// [`post`] with the words given: for a notice that has none of its own (devctl's).
pub(crate) fn post_words(window: &mut Window, notice: Notice, words: String, cx: &mut App) {
    if !super::enabled(cx) {
        return;
    }
    let id = window.window_handle().window_id();
    let entrance = Entrance::of(cx.reduce_motion());
    let theme = cx.global::<SettingsStore>().tokens.appearance;
    let inset = top_inset(window);
    let now = Instant::now();
    // The windows that closed with a chip up are forgotten here, where a chip is next made.
    cx.default_global::<Chips>();
    cx.update_global::<Chips, _>(|chips, cx| {
        let open: Vec<WindowId> = cx.windows().iter().map(|handle| handle.window_id()).collect();
        chips.windows.retain(|window, entry| open.contains(window) || entry.chip.is_none());
        let entry = chips.windows.entry(id).or_default();
        let effect = entry.rules.post(notice, now);
        if matches!(effect, Effect::Ignore) {
            return;
        }
        entry.words = words;
        let content = Content { glyph: notice.glyph(), text: &entry.words, appearance: theme };
        match effect {
            Effect::Show(_) => {
                entry.entrance = Some(entrance);
                if entry.chip.is_none() {
                    entry.chip = Chip::new(window);
                }
                if let Some(chip) = &mut entry.chip {
                    chip.show(&content, inset, entrance);
                }
                if entry.watch.is_none() {
                    let watch = cx.new(|_| Watch);
                    let subscription = watch.update(cx, |_, cx| {
                        cx.observe_window_bounds(window, move |_, window, cx| refit(id, window, cx))
                    });
                    entry.watch = Some((watch, subscription));
                }
            }
            Effect::Swap(_) => {
                if let Some(chip) = &mut entry.chip {
                    chip.swap(&content);
                }
            }
            Effect::Hide | Effect::Remove | Effect::Ignore => {}
        }
        // VoiceOver hears it once, as the chip appears or its words change.
        platform::a11y::announce(&entry.words, false);
        wake(id, entry, cx);
    });
}

/// The window was resized or entered or left full screen: the panel goes under its title row again.
fn refit(id: WindowId, window: &Window, cx: &mut App) {
    let inset = top_inset(window);
    if cx.has_global::<Chips>()
        && let Some(chip) =
            cx.global_mut::<Chips>().windows.get_mut(&id).and_then(|entry| entry.chip.as_mut())
    {
        chip.place(inset);
    }
}

/// The title row's lower edge, from the window's top: the chip rests under it. A full-screen window
/// shows its title row at the edge of the screen, and the chip rests under that edge instead.
fn top_inset(window: &Window) -> f32 {
    if window.is_fullscreen() { 0.0 } else { crate::style::TITLE_ROW_H }
}

/// Set the timer for what the rules next have to do.
fn wake(id: WindowId, entry: &mut Entry, cx: &mut App) {
    entry.timer = entry.rules.due().map(|due| {
        let wait = due.saturating_duration_since(Instant::now());
        cx.spawn(async move |cx| {
            cx.background_executor().timer(wait).await;
            cx.update(|cx| advance(id, cx));
        })
    });
}

/// A life or a leaving is over.
fn advance(id: WindowId, cx: &mut App) {
    if cx.try_global::<Chips>().is_none() {
        return;
    }
    cx.update_global::<Chips, _>(|chips, cx| {
        let Some(entry) = chips.windows.get_mut(&id) else { return };
        let entrance = entry.entrance.unwrap_or(Entrance::Drop);
        match entry.rules.advance(Instant::now(), entrance.leave()) {
            Effect::Hide => {
                if let Some(chip) = &mut entry.chip {
                    chip.hide(entrance);
                }
            }
            Effect::Remove => {
                // The panel goes (dropping it closes it); the window's next message makes another.
                entry.chip = None;
                entry.watch = None;
            }
            Effect::Show(_) | Effect::Swap(_) | Effect::Ignore => {}
        }
        wake(id, entry, cx);
    });
}
