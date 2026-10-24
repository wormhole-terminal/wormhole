//! A check, in debug and test builds, that the chrome GPUI replays is what the window would draw now.
//!
//! A replayed region shows what it drew the last time it was built, so a fact that changes without
//! notifying the workspace leaves it stale until something drops GPUI's caches, and nothing else catches
//! that: a test that draws a window drops them first. Each region therefore records the facts it was built
//! from, and at the end of every frame (`Workspace::check_regions`, from the last probe) the facts of every
//! region the frame placed are read again: a region whose facts differ now was replayed when it should
//! have been built, which is a missing notify. Every window test that draws is such a check.
//!
//! The facts are not the whole of what the builders read. They are what changes without a view of its own
//! to say so: the window's state and the shared state, as text so a failure names what moved. A fact the
//! builders read and this misses is not checked; add it to [`Workspace::region_facts`] when one turns out
//! stale.

use std::cell::Cell;
use std::fmt::Debug;

use gpui_kit::*;

use super::{Part, app_facts::AppFacts};
use wormhole_core::view::panes::Axis;

use crate::window::Workspace;
use crate::window::status_bar::StatusBar;

/// What a region was built from (or is now), each fact by name.
pub(super) type Facts = Vec<(&'static str, String)>;

/// What the regions were built from, and which were placed in the frame being drawn.
#[derive(Default)]
pub(super) struct Oracle {
    recorded: [Option<Facts>; Part::COUNT],
    /// Set while the root is built, by the region being embedded; read and cleared by the check.
    placed: Cell<[bool; Part::COUNT]>,
    /// The window is drawing: from the root's render to the last probe.
    drawing: Cell<bool>,
}

impl Oracle {
    /// The root is being built: a frame begins.
    pub(super) fn begin_frame(&self) {
        self.drawing.set(true);
    }

    /// The window is notified now (`what` did it). GPUI acts on a notification during a draw only for the
    /// views the draw has not reached yet, and runs no observer for it, so the regions (which the workspace's
    /// observer notifies) would never hear of it: a change that needs a redraw is made outside a draw.
    pub(super) fn assert_not_drawing(&self, what: &str) {
        debug_assert!(
            !self.drawing.get(),
            "{what} notified the workspace while it was drawing: no region hears"
        );
    }

    /// `part` is a child of the frame being built.
    pub(super) fn place(&self, part: Part) {
        let mut placed = self.placed.get();
        placed[part.index()] = true;
        self.placed.set(placed);
    }
}

/// `f` without its allocations counted, where they are (tests and devctl builds): the check is not the code
/// it checks, for the tests that count a frame's allocations and `perf-profile`.
#[cfg(any(test, all(debug_assertions, feature = "devctl")))]
use crate::perf::allocations::uncounted;

/// Without a counting allocator there is nothing to leave out.
#[cfg(not(any(test, all(debug_assertions, feature = "devctl"))))]
fn uncounted<R>(f: impl FnOnce() -> R) -> R {
    f()
}

/// `name` among `facts`, as text.
fn fact(facts: &mut Facts, name: &'static str, value: &impl Debug) {
    facts.push((name, format!("{value:?}")));
}

impl Workspace {
    /// What `part` is drawn from, as the window stands.
    ///
    /// `app` is the shared state's facts, read once for the frame: they are the same for every region.
    pub(super) fn region_facts(&self, part: Part, app: &AppFacts, window: &Window, cx: &App) -> Facts {
        let mut facts = Facts::new();
        let columns = self.columns();
        fact(&mut facts, "active", &self.active);
        fact(&mut facts, "app", app);
        fact(&mut facts, "glass", &self.glass_on(cx));
        fact(&mut facts, "slide", &self.slide_motion().map(|motion| motion.edge));
        match part {
            Part::Sidebar => {
                fact(&mut facts, "width", &columns.sidebar);
                fact(&mut facts, "tabs_on", &self.tabs_on_top());
                fact(&mut facts, "profile chip", &Self::profiles_for_chip(cx));
                fact(&mut facts, "odd profile", &self.odd_profile(cx).map(|(profile, _)| profile));
                fact(&mut facts, "no-agents note", &self.no_agents_note_wanted(cx));
            }
            Part::Panel => {
                let tabs = self.panel_tab_list(cx);
                fact(&mut facts, "width", &columns.panel);
                fact(&mut facts, "tab", &self.right_tab);
                fact(&mut facts, "tabs", &tabs);
                fact(&mut facts, "tab words", &self.panel_tab_words(&tabs, cx));
                fact(&mut facts, "editing", &self.feature_editing);
                // What the body is drawn with besides the feature's own entity (which notifies): the row the
                // keys are on, with its ring, and how far the body is scrolled.
                fact(&mut facts, "cursor", &self.frame_facts(window, cx).panel_cursor);
                fact(&mut facts, "panel at", &self.areas.panel_at);
                fact(&mut facts, "scroll", &self.panel_scroll.offset());
            }
            Part::TitleRow => {
                let summary = &self.active_session().pane.summary;
                fact(&mut facts, "covered", &self.covered(cx));
                fact(&mut facts, "pads", &self.title_row_pads(window));
                // The words ("working · 4m") move with the clock and the tick owns them: what they are made
                // of is compared, not the minute they say.
                let mut title = self.title_facts(cx);
                let words =
                    title.words.take().map(|_| crate::window::title::breadcrumb::crumb_source(summary));
                fact(&mut facts, "title", &title);
                fact(&mut facts, "title words", &words);
                fact(&mut facts, "tint", &self.title_tint(cx).map(|tint| (tint.color, tint.peak)));
                fact(&mut facts, "open in", &crate::open_in::OpenIn::resolved(cx));
                fact(&mut facts, "cover file", &self.cover_file(cx));
                fact(&mut facts, "button", &self.areas.title);
                fact(&mut facts, "ports menu", &self.ports_menu);
                fact(&mut facts, "path card", &(self.path_card.is_shown(), self.path_card.is_open_popover()));
                fact(&mut facts, "splits", &(self.can_split(Axis::Row), self.can_split(Axis::Column)));
            }
            Part::StatusBar => {
                // Likewise the bar's state and attention words, and made of what the bar reads.
                let bar = self.status_bar(cx);
                fact(&mut facts, "bar sources", &bar.as_ref().map(|_| self.status_bar_sources(cx)));
                fact(&mut facts, "bar", &bar.map(StatusBar::without_clock));
                fact(&mut facts, "item", &self.areas.status);
            }
        }
        facts
    }

    /// The panel's tabs as VoiceOver and the badges say them.
    fn panel_tab_words(
        &self,
        tabs: &[wormhole_core::config::TabId],
        cx: &App,
    ) -> Vec<(Option<usize>, String)> {
        let facts = self.facts(cx);
        tabs.iter()
            .filter_map(|&id| {
                let (at, def) = self.tab_entry(id)?;
                let entity = self.feature_entity(at)?;
                Some((
                    def.count.and_then(|count| count(entity, &facts, cx)),
                    (def.spoken)(entity, &facts, cx),
                ))
            })
            .collect()
    }

    /// Whether the sidebar draws its note after a first run without agents, and what it reads.
    fn no_agents_note_wanted(&self, cx: &App) -> (bool, bool) {
        let dismissed = crate::app_state::app(cx).tips.contains(wormhole_core::onboarding::Tip::NoAgents);
        let agent_seen = self.sessions.iter().any(|session| session.pane.summary.agent.is_some());
        (self.first_run.read(cx).note_wanted(dismissed), agent_seen)
    }

    /// Remember what `part` was just built from.
    pub(super) fn record_region(&mut self, part: Part, window: &Window, cx: &App) {
        // Not the code it checks, for the tests that count a frame's allocations.
        let facts = uncounted(|| self.region_facts(part, &AppFacts::of(self, cx), window, cx));
        self.regions.oracle.recorded[part.index()] = Some(facts);
    }

    /// At the end of a frame: every region it placed shows what the window holds now.
    pub(in crate::window) fn check_regions(&mut self, window: &Window, cx: &App) {
        uncounted(|| self.check_placed_regions(window, cx));
    }

    fn check_placed_regions(&mut self, window: &Window, cx: &App) {
        self.regions.oracle.drawing.set(false);
        let placed = self.regions.oracle.placed.take();
        let mut stale = Vec::new();
        let app = AppFacts::of(self, cx);
        for part in Part::ALL {
            if !placed[part.index()] {
                continue;
            }
            let Some(drawn) = self.regions.oracle.recorded[part.index()].take() else { continue };
            let now = self.region_facts(part, &app, window, cx);
            let moved: Vec<String> = drawn
                .iter()
                .zip(&now)
                .filter(|(was, is)| was != is)
                .map(|((name, was), (_, is))| format!("{name}: {was} -> {is}"))
                .collect();
            // Remembered as it stands now, so a stale region is reported once and not in every frame.
            self.regions.oracle.recorded[part.index()] = Some(now);
            if !moved.is_empty() {
                stale.push(format!(
                    "{part:?} of window {} was replayed with facts that changed:\n  {}",
                    self.window_id,
                    moved.join("\n  ")
                ));
            }
        }
        if stale.is_empty() {
            return;
        }
        let report = format!(
            "a region shows what the window no longer holds (a fact changed without a notify)\n{}",
            stale.join("\n")
        );
        if cfg!(test) {
            panic!("{report}");
        }
        tracing::error!("{report}");
    }
}
