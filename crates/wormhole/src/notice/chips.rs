//! Each window's chip: core's rules, the platform's panel and the timer that moves them on, kept in a global
//! keyed by the window. Nothing here draws; a window with no native panel (a test) still runs the rules, so
//! what was said and when can be asked of it.
//!
//! Every post names its window and, for a notice about one terminal, that terminal (an [`Anchor`]); the work is
//! done a turn later, outside any update of the poster, so the window's own state can be read then. The
//! clock is GPUI's, so a test moves it.

use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::storage::paths::ProfileId;
use wormhole_core::view::motion::notice::Entrance;
use wormhole_core::view::notice::{ChipRules, Effect, Notice, Spot};
use wormhole_core::view::panes::Rect;

use crate::platform;
use crate::platform::notice::{Chip, Content};
use crate::settings::{settings_in, store_in};

/// How a terminal's bounds in its window are read again whenever the chip is placed: a chip about a
/// terminal that moves (a split dragged, a window resized) follows it.
pub(crate) type Anchor = Rc<dyn Fn(&App) -> Option<Bounds<Pixels>>>;

/// What a window tells the chips about itself.
pub(crate) struct Facts {
    /// The profile whose settings and theme the window follows.
    pub(crate) profile: ProfileId,
    /// Where its selected terminal is, for a notice that is not about a terminal of its own.
    pub(crate) selected: Option<Bounds<Pixels>>,
}

/// A window's way of telling [`Facts`], read when a notice is said or placed.
pub(crate) type Home = Rc<dyn Fn(&App) -> Option<Facts>>;

/// How long after the window last changed its size the chip is placed once more: a full-screen change
/// reports the size before it reports the state, and the transition takes a moment.
const SETTLE: Duration = Duration::from_millis(250);

/// One window's chip.
#[derive(Default)]
pub(super) struct Entry {
    pub(super) rules: ChipRules,
    /// The panel, while there is one (from the first message until the chip is out of sight).
    pub(super) chip: Option<Chip>,
    /// What the chip says now (or said last).
    pub(super) words: String,
    /// Where the chip is about, as the poster said.
    anchor: Option<Anchor>,
    /// Where the chip was last put.
    pub(super) spot: Option<Spot>,
    /// How it came, so it goes the same way.
    entrance: Option<Entrance>,
    /// Wakes the rules when a life or a leaving is over; dropping it cancels it.
    timer: Option<Task<()>>,
    /// Places the chip once more when the window has stopped changing; dropping it cancels it.
    settle: Option<Task<()>>,
    /// Puts the chip back at its terminal when the window is resized or goes full screen. GPUI hands that
    /// out to an entity's context, so a small one holds it, and with it the subscription.
    watch: Option<(Entity<Watch>, Subscription)>,
}

/// What listens to the window's size for its chip.
struct Watch;

#[derive(Default)]
pub(super) struct Chips {
    pub(super) windows: HashMap<WindowId, Entry>,
    homes: HashMap<WindowId, Home>,
    /// Hears every window close, for the chips and homes of the closed.
    closed: Option<Subscription>,
}

impl Global for Chips {}

/// The window `window` tells the chips about itself with `home`, for as long as it is open.
pub(crate) fn register(window: &Window, home: Home, cx: &mut App) {
    let id = window.window_handle().window_id();
    cx.default_global::<Chips>();
    cx.update_global::<Chips, _>(|chips, cx| {
        chips.homes.insert(id, home);
        if chips.closed.is_none() {
            chips.closed = Some(cx.on_window_closed(close));
        }
    });
}

/// A window closed: its chip goes with it at once, and nothing is kept of it.
fn close(cx: &mut App, id: WindowId) {
    cx.update_global::<Chips, _>(|chips, cx| {
        chips.homes.remove(&id);
        if let Some(entry) = chips.windows.remove(&id)
            && let Some(chip) = entry.chip
        {
            chip.close(cx);
        }
    });
}

/// Whether the chip is on for a window of `profile`: its feature flag. Off, a copy says nothing (and a
/// program's clipboard write keeps the pane bar's notice, which the chip replaces when it is on).
pub(crate) fn enabled(profile: &ProfileId, cx: &App) -> bool {
    settings_in(profile, cx).feature_enabled(FeatureFlag::Notices)
}

/// Say `notice` in `window`, about the terminal `anchor` finds (`None`: the window's selected one).
pub(crate) fn post(window: &Window, notice: impl Into<Notice>, anchor: Option<Anchor>, cx: &mut App) {
    post_to(window.window_handle(), notice, anchor, cx);
}

/// [`post`] for a poster that has the window's handle and not the window.
pub(crate) fn post_to(
    window: AnyWindowHandle,
    notice: impl Into<Notice>,
    anchor: Option<Anchor>,
    cx: &mut App,
) {
    queue(window, notice.into(), None, anchor, cx);
}

/// Say `notice` with `words` of its own (devctl's).
#[cfg(feature = "devctl")]
pub(super) fn post_words(window: &Window, notice: Notice, words: String, cx: &mut App) {
    queue(window.window_handle(), notice, Some(words), None, cx);
}

fn queue(
    window: AnyWindowHandle,
    notice: Notice,
    words: Option<String>,
    anchor: Option<Anchor>,
    cx: &mut App,
) {
    cx.defer(move |cx| {
        window.update(cx, |_, window, cx| say(window, notice, words, anchor, cx)).ok();
    });
}

/// Say it, now that nothing of the poster is borrowed.
fn say(window: &mut Window, notice: Notice, words: Option<String>, anchor: Option<Anchor>, cx: &mut App) {
    let id = window.window_handle().window_id();
    // A window that has not said who it is gets nothing: better no chip than one in the wrong place.
    let Some(facts) =
        cx.try_global::<Chips>().and_then(|chips| chips.homes.get(&id)).and_then(|home| home(cx))
    else {
        return;
    };
    if !enabled(&facts.profile, cx) {
        return;
    }
    let Some(words) = words.or_else(|| super::words::of(notice)) else { return };
    let store = store_in(&facts.profile, cx);
    let (theme, opaque) = (store.tokens.appearance, store.system().opaque());
    let entrance = Entrance::of(cx.reduce_motion());
    let spot = spot_for(window, anchor.as_ref().and_then(|anchor| anchor(cx)).or(facts.selected));
    let now = cx.background_executor().now();
    cx.update_global::<Chips, _>(|chips, cx| {
        // The windows that closed with a chip up are gone already (`close`); this is for one a test closed.
        let entry = chips.windows.entry(id).or_default();
        let effect = entry.rules.post(notice, now);
        if matches!(effect, Effect::Ignore) {
            return;
        }
        entry.words = words;
        entry.anchor = anchor;
        entry.spot = Some(spot);
        let content = Content { glyph: notice.glyph(), text: &entry.words, appearance: theme };
        match effect {
            Effect::Show => {
                entry.entrance = Some(entrance);
                if entry.chip.is_none() {
                    entry.chip = Chip::new(window, opaque);
                }
                if let Some(chip) = &mut entry.chip {
                    chip.show(&content, spot, entrance, cx);
                }
                if entry.watch.is_none() {
                    let watch = cx.new(|_| Watch);
                    let handle = window.window_handle();
                    let subscription = watch.update(cx, |_, cx| {
                        cx.observe_window_bounds(window, move |_, _, cx| resized(handle, cx))
                    });
                    entry.watch = Some((watch, subscription));
                }
            }
            Effect::Swap => {
                if let Some(chip) = &mut entry.chip {
                    chip.swap(&content, spot);
                }
            }
            Effect::Hide | Effect::Remove | Effect::Ignore => {}
        }
        // VoiceOver hears it once, as the chip appears or its words change.
        platform::a11y::announce(&entry.words, false);
        wake(id, entry, cx);
    });
}

/// Where the chip goes for a terminal at `bounds`: its top edge, centred on it; where it is not known (a
/// window that has not drawn, a test), the whole window below the title row.
fn spot_for(window: &Window, bounds: Option<Bounds<Pixels>>) -> Spot {
    let view = window.viewport_size();
    let (w, h) = (f32::from(view.width), f32::from(view.height));
    let title_row = crate::style::TITLE_ROW_H;
    let anchor = bounds.filter(|bounds| bounds.size.width > px(0.0) && bounds.size.height > px(0.0)).map_or(
        Rect::new(0.0, title_row, w, (h - title_row).max(0.0)),
        |bounds| {
            Rect::new(
                f32::from(bounds.origin.x),
                f32::from(bounds.origin.y),
                f32::from(bounds.size.width),
                f32::from(bounds.size.height),
            )
        },
    );
    Spot { window: (w, h), anchor, full_screen: window.is_fullscreen(), title_row }
}

/// The window changed its size or went in or out of full screen: the chip goes to its terminal again, and
/// once more when the window has been still for a moment (the state of full screen can follow the size).
fn resized(window: AnyWindowHandle, cx: &mut App) {
    cx.defer(move |cx| {
        window.update(cx, |_, window, cx| refit(window, cx)).ok();
        let id = window.window_id();
        cx.update_global::<Chips, _>(|chips, cx| {
            if let Some(entry) = chips.windows.get_mut(&id) {
                entry.settle = Some(cx.spawn(async move |cx| {
                    cx.background_executor().timer(SETTLE).await;
                    cx.update(|cx| {
                        window.update(cx, |_, window, cx| refit(window, cx)).ok();
                    });
                }));
            }
        });
    });
}

/// Put the window's chip at its terminal's top edge again.
fn refit(window: &mut Window, cx: &mut App) {
    let id = window.window_handle().window_id();
    let Some(chips) = cx.try_global::<Chips>() else { return };
    let Some(entry) = chips.windows.get(&id).filter(|entry| entry.chip.is_some()) else { return };
    let bounds = entry
        .anchor
        .as_ref()
        .and_then(|anchor| anchor(cx))
        .or_else(|| chips.homes.get(&id).and_then(|home| home(cx)).and_then(|facts| facts.selected));
    let spot = spot_for(window, bounds);
    if let Some(entry) = cx.global_mut::<Chips>().windows.get_mut(&id)
        && let Some(chip) = entry.chip.as_mut()
    {
        chip.place(spot);
        entry.spot = Some(spot);
    }
}

/// Set the timer for what the rules next have to do.
fn wake(id: WindowId, entry: &mut Entry, cx: &mut App) {
    let now = cx.background_executor().now();
    entry.timer = entry.rules.due().map(|due| {
        let wait = due.saturating_duration_since(now);
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
    let now = cx.background_executor().now();
    cx.update_global::<Chips, _>(|chips, cx| {
        let Some(entry) = chips.windows.get_mut(&id) else { return };
        let entrance = entry.entrance.unwrap_or(Entrance::Drop);
        match entry.rules.advance(now, entrance.exit_duration()) {
            Effect::Hide => {
                if let Some(chip) = &mut entry.chip {
                    chip.hide(entrance);
                }
            }
            Effect::Remove => {
                // The panel goes; the window's next message makes another.
                if let Some(chip) = entry.chip.take() {
                    chip.close(cx);
                }
                entry.watch = None;
                entry.settle = None;
            }
            Effect::Show | Effect::Swap | Effect::Ignore => {}
        }
        wake(id, entry, cx);
    });
}
