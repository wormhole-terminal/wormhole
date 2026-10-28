//! Each window's chip: core's rules, the platform's panel and the timer that moves them on, kept in a global
//! keyed by the window. Nothing here draws; a window with no native panel (a test) still runs the rules, so
//! what was said and when can be asked of it.
//!
//! A sticky notice (a warning) stays until the person clicks the chip: the platform's panel says so over a
//! channel, a task here reads it and asks the rules to dismiss.
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
use wormhole_core::theme::tokens::Appearance;
use wormhole_core::view::motion::notice::Entrance;
use wormhole_core::view::notice::{Buttons, ChipRules, Effect, Glyph, Notice, Press, Reaction, Spot};
use wormhole_core::view::panes::Rect;
use wormhole_text::{keys, text};

use crate::platform;
use crate::platform::notice::{Chip, Clicks, Content};
use crate::platform::popover::Handle;
use crate::settings::{settings_in, store_in};

/// How a terminal's bounds in its window are read again whenever the chip is placed: a chip about a
/// terminal that moves (a split dragged, a window resized) follows it. It holds the terminal weakly: a
/// closed terminal is [`Where::Gone`], and nothing here keeps it (or its shell) alive.
pub(crate) type Anchor = Rc<dyn Fn(&App) -> Where>;

/// Where the terminal a notice is about is.
pub(crate) enum Where {
    /// In its window at `bounds` as the last frame drew it. `terminal` is its entity, which the window's
    /// [`Facts::shown`] names when it is on screen.
    At { bounds: Bounds<Pixels>, terminal: EntityId },
    /// Closed.
    Gone,
}

impl Where {
    fn bounds(&self) -> Option<Bounds<Pixels>> {
        match self {
            Self::At { bounds, .. } => Some(*bounds),
            Self::Gone => None,
        }
    }
}

/// What a window tells the chips about itself.
pub(crate) struct Facts {
    /// The profile whose settings and theme the window follows.
    pub(crate) profile: ProfileId,
    /// Where its selected terminal is, for a notice that is not about a terminal of its own.
    pub(crate) selected: Option<Bounds<Pixels>>,
    /// The terminals on screen (the selected one and the panes shown with it), by entity.
    pub(crate) shown: Vec<EntityId>,
    /// Something is drawn over the content (the command palette, the shortcut sheet, the folder picker): a
    /// chip taking clicks would take them from it.
    pub(crate) covered: bool,
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
    /// The theme's appearance the chip was last drawn in, for words put back without a post.
    pub(super) theme: Option<Appearance>,
    /// The picture beside the words now, for the same.
    pub(super) glyph: Option<Glyph>,
    /// The look the chip was made in: the theme's appearance and whether the system asks for a plain fill.
    pub(super) look: Option<(Appearance, bool)>,
    /// The panel is out while the sticky chip's terminal is off screen or covered (`follow`).
    pub(super) hidden: bool,
    /// Keeps a sticky chip at its terminal; dropping it stops it.
    pub(super) follow: Option<Task<()>>,
    /// Where the chip is about, as the poster said.
    pub(super) anchor: Option<Anchor>,
    /// What the sticky notice says, kept for when a copy's words give way to it.
    pub(super) sticky_words: Option<String>,
    /// What the sticky notice's explanation says went wrong, in the error's own words (core's `reason`).
    pub(super) sticky_detail: Option<String>,
    /// The explanation, while its popover is open: which opening it is, and how to close it from here.
    pub(super) info: Option<(u64, Handle)>,
    /// How many have been opened, so a close that comes late is not taken for the newest's.
    pub(super) opened: u64,
    /// When the explanation last closed by itself (a click outside it): the click on ⓘ that closed it is
    /// the same click that would open it again.
    pub(super) info_closed: Option<std::time::Instant>,
    /// Where the sticky notice is about: its chip goes back there when a copy's words give way to it.
    pub(super) sticky_anchor: Option<Anchor>,
    /// Where the panel says it was clicked, once there is a panel; the task reads the other end.
    pub(super) clicks: Option<Clicks>,
    /// Dismisses the chip when it is clicked; dropping it stops listening.
    listener: Option<Task<()>>,
    /// Where the chip was last put.
    pub(super) spot: Option<Spot>,
    /// How it came, so it goes the same way.
    pub(super) entrance: Option<Entrance>,
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
    pub(super) homes: HashMap<WindowId, Home>,
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
        if let Some(mut entry) = chips.windows.remove(&id) {
            if let Some((_, info)) = entry.info.take() {
                info.close(cx);
            }
            if let Some(chip) = entry.chip.take() {
                chip.close(cx);
            }
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
    queue(window, notice.into(), None, anchor, None, cx);
}

/// [`post_to`] for a notice with an explanation (`Notice::has_info`), which says `detail` of what went
/// wrong: an error's own words, never anything a terminal showed.
pub(crate) fn post_to_with(
    window: AnyWindowHandle,
    notice: impl Into<Notice>,
    anchor: Option<Anchor>,
    detail: String,
    cx: &mut App,
) {
    queue(window, notice.into(), None, anchor, Some(detail), cx);
}

/// Say `notice` with `words` of its own (devctl's).
#[cfg(feature = "devctl")]
pub(super) fn post_words(window: &Window, notice: Notice, words: String, cx: &mut App) {
    queue(window.window_handle(), notice, Some(words), None, None, cx);
}

/// Say a notice that has words in the catalog, with `detail` for its explanation (devctl's `notice sticky`).
#[cfg(feature = "devctl")]
pub(super) fn post_notice(window: &Window, notice: Notice, detail: Option<String>, cx: &mut App) {
    queue(window.window_handle(), notice, None, None, detail, cx);
}

/// Click the chip of `window` as the panel does: the click goes over the same channel. `false` when the
/// window has no chip yet (a chip in a test has no panel, and the channel is the same).
#[cfg(any(test, feature = "devctl"))]
pub(super) fn press(window: WindowId, press: Press, cx: &App) -> bool {
    cx.try_global::<Chips>()
        .and_then(|chips| chips.windows.get(&window))
        .and_then(|entry| entry.clicks.as_ref())
        .is_some_and(|clicks| clicks.try_send(press).is_ok())
}

fn queue(
    window: AnyWindowHandle,
    notice: Notice,
    words: Option<String>,
    anchor: Option<Anchor>,
    detail: Option<String>,
    cx: &mut App,
) {
    cx.defer(move |cx| {
        window.update(cx, |_, window, cx| say(window, notice, words, anchor, detail, cx)).ok();
    });
}

/// Say it, now that nothing of the poster is borrowed.
fn say(
    window: &mut Window,
    notice: Notice,
    words: Option<String>,
    anchor: Option<Anchor>,
    detail: Option<String>,
    cx: &mut App,
) {
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
    let placed = anchor.as_ref().map(|anchor| anchor(cx));
    let spot = spot_for(window, placed.as_ref().and_then(Where::bounds).or(facts.selected));
    // A warning about a terminal that is not on screen, or under an overlay, waits out of sight.
    let hidden = notice.is_sticky() && super::follow::hidden(&facts, placed.as_ref());
    let now = cx.background_executor().now();
    cx.update_global::<Chips, _>(|chips, cx| {
        // The windows that closed with a chip up are gone already (`close`); this is for one a test closed.
        let entry = chips.windows.entry(id).or_default();
        let effect = entry.rules.post(notice, now);
        if matches!(effect, Effect::Ignore) {
            return;
        }
        if notice.is_sticky() {
            entry.sticky_words = Some(words.clone());
            entry.sticky_anchor = anchor.clone();
            entry.sticky_detail = detail.map(|detail| wormhole_core::view::notice::reason(&detail));
        }
        entry.words = words;
        entry.theme = Some(theme);
        entry.glyph = Some(notice.glyph());
        entry.look = Some((theme, opaque));
        entry.anchor = anchor;
        entry.spot = Some(spot);
        let content = content(notice.glyph(), &entry.words, theme, entry.rules.buttons());
        match effect {
            Effect::Show => {
                entry.entrance = Some(entrance);
                if entry.clicks.is_none() {
                    let (clicks, listener) = listen(id, cx);
                    entry.clicks = Some(clicks);
                    entry.listener = Some(listener);
                }
                if entry.chip.is_none()
                    && let Some(clicks) = entry.clicks.clone()
                {
                    entry.chip = Chip::new(window, opaque, clicks);
                }
                entry.hidden = false;
                if let Some(chip) = &mut entry.chip {
                    chip.show(&content, spot, entrance, entry.rules.clickable(), cx);
                    if hidden {
                        chip.set_hidden(true, cx);
                        entry.hidden = true;
                    }
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
                    chip.swap(&content, spot, entry.rules.clickable());
                    // A copy is about a terminal in view, and a new warning says itself where it is wanted.
                    if entry.hidden != hidden {
                        chip.set_hidden(hidden, cx);
                        entry.hidden = hidden;
                    }
                }
            }
            Effect::Hide | Effect::Remove | Effect::Restore | Effect::Ignore => {}
        }
        // VoiceOver hears it once, as the chip appears or its words change.
        platform::a11y::announce(&spoken(notice, &entry.words), false);
        if entry.rules.held().is_some() && entry.follow.is_none() {
            entry.follow = Some(super::follow::watch(window.window_handle(), cx));
        }
        wake(id, entry, cx);
    });
}

/// Forget which sticky notices of `window` were dismissed, so devctl can show one again.
#[cfg(feature = "devctl")]
pub(super) fn forget_dismissed(window: WindowId, cx: &mut App) {
    if cx.try_global::<Chips>().is_none() {
        return;
    }
    if let Some(entry) = cx.global_mut::<Chips>().windows.get_mut(&window) {
        entry.rules.forget_dismissed();
    }
}

/// What the chip shows: `glyph`, `words`, the look of the theme, and the buttons with their names.
pub(super) fn content<'a>(glyph: Glyph, words: &'a str, theme: Appearance, buttons: Buttons) -> Content<'a> {
    Content {
        glyph,
        text: words,
        appearance: theme,
        buttons,
        close_label: text(keys::NOTICE_CLOSE),
        info_label: text(keys::NOTICE_INFO),
    }
}

/// What VoiceOver says for `notice`, whose words on the chip are `words`.
fn spoken(notice: Notice, words: &str) -> String {
    super::words::spoken(notice).unwrap_or_else(|| words.to_string())
}

/// The channel a window's panel says its clicks over, and the task that dismisses the chip on each.
fn listen(id: WindowId, cx: &mut App) -> (Clicks, Task<()>) {
    let (clicks, heard) = smol::channel::unbounded();
    let listener = cx.spawn(async move |cx| {
        while let Ok(press) = heard.recv().await {
            cx.update(|cx| pressed(id, press, cx));
        }
    });
    (clicks, listener)
}

/// The person clicked the chip at `press`: the rules say what that comes to for the buttons it has.
pub(super) fn pressed(id: WindowId, press: Press, cx: &mut App) {
    let reaction = cx
        .try_global::<Chips>()
        .and_then(|chips| chips.windows.get(&id))
        .map_or(Reaction::Nothing, |entry| entry.rules.buttons().react(press));
    match reaction {
        Reaction::Dismiss => dismiss(id, cx),
        Reaction::ShowInfo => {
            if let Some(window) = cx.windows().into_iter().find(|window| window.window_id() == id) {
                window.update(cx, |_, window, cx| super::explain::toggle(window, cx)).ok();
            }
        }
        Reaction::Nothing => {}
    }
}

/// The × was clicked: a sticky notice goes, and the rules say whether this chip had one to lose.
pub(super) fn dismiss(id: WindowId, cx: &mut App) {
    let now = cx.background_executor().now();
    take_away(id, cx, |rules, leave| rules.dismiss(now, leave));
}

/// What the sticky notice is about is gone: the chip takes it away.
pub(super) fn withdraw(id: WindowId, cx: &mut App) {
    let now = cx.background_executor().now();
    take_away(id, cx, |rules, leave| rules.withdraw(now, leave));
}

/// Let the rules take the sticky notice away (`how`), and when they do, send the chip off and let go of
/// everything it held about its terminal.
fn take_away(id: WindowId, cx: &mut App, how: impl FnOnce(&mut ChipRules, Duration) -> Effect) {
    if cx.try_global::<Chips>().is_none() {
        return;
    }
    cx.update_global::<Chips, _>(|chips, cx| {
        let Some(entry) = chips.windows.get_mut(&id) else { return };
        let entrance = entry.entrance.unwrap_or(Entrance::Drop);
        if matches!(how(&mut entry.rules, entrance.exit_duration()), Effect::Hide) {
            entry.sticky_words = None;
            entry.sticky_anchor = None;
            entry.sticky_detail = None;
            entry.anchor = None;
            entry.follow = None;
            if let Some((_, info)) = entry.info.take() {
                info.close(cx);
            }
            if let Some(chip) = &mut entry.chip {
                chip.set_clickable(false);
                chip.hide(entrance);
            }
        }
        wake(id, entry, cx);
    });
}

/// Where the chip goes for a terminal at `bounds`: its top edge, centred on it; where it is not known (a
/// window that has not drawn, a test), the whole window below the title row.
pub(super) fn spot_for(window: &Window, bounds: Option<Bounds<Pixels>>) -> Spot {
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
        .and_then(|anchor| anchor(cx).bounds())
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
    let mut restored = false;
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
                entry.follow = None;
                entry.hidden = false;
                if let Some((_, info)) = entry.info.take() {
                    info.close(cx);
                }
            }
            Effect::Restore => {
                restore(entry);
                restored = true;
            }
            Effect::Show | Effect::Swap | Effect::Ignore => {}
        }
        wake(id, entry, cx);
    });
    if restored {
        // A copy about another terminal may have moved the chip: the sticky notice goes back to its own.
        if let Some(window) = cx.windows().into_iter().find(|window| window.window_id() == id) {
            cx.defer(move |cx| {
                window.update(cx, |_, window, cx| refit(window, cx)).ok();
            });
        }
    }
}

/// A copy's words are done and the sticky ones come back, with the glass it was in.
fn restore(entry: &mut Entry) {
    let Some(held) = entry.rules.held() else { return };
    let (Some(words), Some(theme)) = (entry.sticky_words.clone(), entry.theme) else { return };
    entry.words = words;
    entry.anchor = entry.sticky_anchor.clone();
    entry.glyph = Some(held.glyph());
    let content = content(held.glyph(), &entry.words, theme, Buttons::of(held));
    if let (Some(chip), Some(spot)) = (&mut entry.chip, entry.spot) {
        chip.swap(&content, spot, true);
    }
    platform::a11y::announce(&spoken(held, &entry.words), false);
}
