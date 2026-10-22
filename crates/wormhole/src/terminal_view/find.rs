//! Find in scrollback for one terminal: the query, what it found and which match is
//! current. The search runs off the UI thread on a copy of the grid's text (core
//! `terminal::find`); while find is open and the program keeps printing, it runs again at most
//! every [`RESEARCH_EVERY`]. In between, a match is drawn where the output has moved it (the
//! history grew by so many lines) and only while its cells still hold what it matched, so a
//! redrawn screen never shows a stale highlight.

use std::sync::Arc;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::limits::count_grouped;
use wormhole_core::terminal::ScreenSnapshot;
use wormhole_core::terminal::find::{FindOption, FindQuery, FindResults};
use wormhole_core::terminal::types::Line;

use super::TerminalView;
use super::find_bar::FindBar;
use super::status::TerminalEvent;

/// How often a terminal that keeps printing is searched again while its find bar is open.
const RESEARCH_EVERY: Duration = Duration::from_millis(200);

/// Find's state in one terminal.
#[derive(Default)]
pub(super) struct Find {
    open: bool,
    query: FindQuery,
    /// The bar, made the first time find opens (it needs a window); kept with its text after.
    bar: Option<Entity<FindBar>>,
    results: Arc<FindResults>,
    /// The history's length the results are as of.
    history: usize,
    /// The output revision the results are as of.
    revision: u64,
    current: Option<usize>,
    /// The query is a regular expression that doesn't parse.
    invalid: bool,
    /// The search running, or the wait before searching again after output.
    task: Option<Task<()>>,
    /// Bumped by each search: a result from an older one is dropped.
    generation: u64,
}

/// What the bar shows.
pub(super) struct FindStatus {
    pub count: SharedString,
    pub invalid: bool,
    pub options: wormhole_core::terminal::find::FindOptions,
}

impl TerminalView {
    /// The find bar, while it is open.
    pub fn find_bar(&self) -> Option<Entity<FindBar>> {
        self.find.bar.clone().filter(|_| self.find.open)
    }

    pub fn find_open(&self) -> bool {
        self.find.open
    }

    /// The query and what it found, for devctl's `state`.
    #[cfg(any(test, feature = "devctl"))]
    pub fn find_summary(&self) -> Option<String> {
        self.find.open.then(|| format!("{:?} {}", self.find.query.text, self.find_status().count))
    }

    /// ⌘F: open the bar with the keyboard in its field, its text selected. `seed` replaces the
    /// query (the selection).
    pub fn open_find(&mut self, seed: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.show_find(seed, window, cx);
        if let Some(bar) = &self.find.bar {
            bar.update(cx, |bar, cx| bar.focus(window, cx));
        }
    }

    /// ⌘E: the selection becomes the query; the keyboard stays where it is. Nothing selected,
    /// nothing changes.
    pub fn use_selection_for_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(seed) = self.find_seed() {
            self.show_find(Some(seed), window, cx);
        }
    }

    /// The selection as a query: its first line (a find doesn't cross line breaks).
    pub fn find_seed(&self) -> Option<String> {
        let selection = self.selection_text()?;
        let line = selection.lines().find(|line| !line.trim().is_empty())?;
        Some(line.to_string())
    }

    fn show_find(&mut self, seed: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        let weak = cx.entity().downgrade();
        let bar = self.find.bar.get_or_insert_with(|| cx.new(|cx| FindBar::new(weak, window, cx))).clone();
        let reopened = !self.find.open;
        self.find.open = true;
        match seed {
            Some(seed) if seed != self.find.query.text => {
                bar.update(cx, |bar, cx| bar.set_text(&seed, window, cx));
                self.find.query.text = seed;
                self.search(true, cx);
            }
            // The query it was closed with finds again, from where the view is now.
            _ if reopened => self.search(true, cx),
            _ => {}
        }
        cx.emit(TerminalEvent::FindChanged);
        cx.notify();
    }

    /// Close the bar; the keyboard goes back to the terminal.
    pub fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.find.open {
            return;
        }
        self.find.open = false;
        self.find.task = None;
        self.find.results = Arc::default();
        self.find.current = None;
        window.focus(&self.focus_handle, cx);
        cx.emit(TerminalEvent::FindChanged);
        cx.notify();
    }

    /// ⌘G / ⇧⌘G (and ↩ / ⇧↩ in the bar): the next or previous match, round the ends. With find
    /// closed or nothing typed, the bar opens instead, as ⌘F would.
    pub fn find_step(&mut self, delta: isize, window: &mut Window, cx: &mut Context<Self>) {
        if !self.find.open || self.find.query.text.is_empty() {
            self.open_find(None, window, cx);
            return;
        }
        let len = self.find.results.matches.len();
        if len == 0 {
            return;
        }
        let next = match self.find.current {
            Some(current) => (current as isize + delta).rem_euclid(len as isize) as usize,
            None if delta < 0 => len - 1,
            None => 0,
        };
        self.find.current = Some(next);
        self.reveal_current();
        self.find_changed(cx);
    }

    /// The bar's field changed.
    pub(super) fn set_find_text(&mut self, text: String, cx: &mut Context<Self>) {
        if text != self.find.query.text {
            self.find.query.text = text;
            self.search(true, cx);
        }
    }

    pub(super) fn toggle_find_option(&mut self, option: FindOption, cx: &mut Context<Self>) {
        self.find.query.options.toggle(option);
        self.search(true, cx);
    }

    pub(super) fn find_status(&self) -> FindStatus {
        let find = &self.find;
        let total = find.results.matches.len();
        let count = if find.invalid {
            "Not a valid pattern".to_string()
        } else if find.query.text.is_empty() || find.task.is_some() && total == 0 {
            String::new()
        } else if total == 0 {
            "No results".to_string()
        } else {
            let plus = if find.results.capped { "+" } else { "" };
            let current = find.current.map_or(0, |ix| ix + 1);
            format!("{} of {}{plus}", count_grouped(current), count_grouped(total))
        };
        FindStatus { count: count.into(), invalid: find.invalid, options: find.query.options }
    }

    /// Search for the query. `fresh`: the query changed (or find opened), so the current match is
    /// picked again near the view and scrolled to; otherwise (the terminal printed) the current
    /// match stays and the view doesn't move.
    fn search(&mut self, fresh: bool, cx: &mut Context<Self>) {
        self.find.generation += 1;
        let generation = self.find.generation;
        let regex = self.find.query.compile();
        self.find.invalid = regex.is_err();
        let (Some(pty), Ok(Some(regex))) = (&self.pty, regex) else {
            self.find.task = None;
            self.find.results = Arc::default();
            self.find.current = None;
            self.find_changed(cx);
            return;
        };
        let source = pty.scrollback();
        self.find.task = Some(cx.spawn(async move |this, cx| {
            let found = cx
                .background_spawn(async move {
                    let (text, revision) = source.grid_text();
                    (text.find(&regex), text.history(), revision)
                })
                .await;
            let _ = this.update(cx, |this, cx| this.found(generation, found, fresh, cx));
        }));
        // "No results" waits for the search, so typing doesn't flash it.
        self.find_changed(cx);
    }

    fn found(
        &mut self,
        generation: u64,
        (results, history, revision): (FindResults, usize, u64),
        fresh: bool,
        cx: &mut Context<Self>,
    ) {
        if generation != self.find.generation || !self.find.open {
            return;
        }
        let len = results.matches.len();
        self.find.results = Arc::new(results);
        self.find.history = history;
        self.find.revision = revision;
        self.find.task = None;
        if fresh {
            let (_, rows) = self.grid_size();
            let bottom = Line(rows as i32 - 1 - self.render_state.borrow().display_offset() as i32);
            self.find.current = self.find.results.nearest(bottom + self.history_growth());
            self.reveal_current();
        } else {
            self.find.current = self.find.current.map(|ix| ix.min(len.saturating_sub(1))).filter(|_| len > 0);
        }
        self.find_changed(cx);
        // Printed again while it searched.
        if self.pty.as_ref().is_some_and(|pty| pty.output_revision() != revision) {
            self.research_soon(cx);
        }
    }

    /// The program printed: while find is open, search again soon (once per [`RESEARCH_EVERY`]
    /// however much it prints, and never two at once).
    pub(super) fn find_output(&mut self, cx: &mut Context<Self>) {
        let printed = self.pty.as_ref().is_some_and(|pty| pty.output_revision() != self.find.revision);
        let searching = !self.find.query.text.is_empty() && !self.find.invalid;
        if self.find.open && printed && searching && self.find.task.is_none() {
            self.research_soon(cx);
        }
    }

    fn research_soon(&mut self, cx: &mut Context<Self>) {
        self.find.task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(RESEARCH_EVERY).await;
            let _ = this.update(cx, |this, cx| this.search(false, cx));
        }));
    }

    /// How many lines the history grew since the search: the matches have moved up that far.
    fn history_growth(&self) -> i32 {
        let now = self.render_state.borrow().history_size();
        now.saturating_sub(self.find.history) as i32
    }

    /// Scroll the current match into view.
    fn reveal_current(&self) {
        let Some(m) = self.find.current.and_then(|ix| self.find.results.matches.get(ix)) else { return };
        if let Some(pty) = &self.pty {
            pty.reveal(m.start.line - self.history_growth());
        }
    }

    /// The bar's count or the highlights changed: draw both again.
    fn find_changed(&mut self, cx: &mut Context<Self>) {
        // By id: the bar may be the one being updated (its field changed the query).
        if let Some(bar) = &self.find.bar {
            App::notify(cx, bar.entity_id());
        }
        // The view is cached: paint the highlights again.
        cx.notify();
    }

    /// Find's matches on each viewport row of `snapshot`: their columns, and whether each is
    /// the current one. Empty while find is closed.
    pub(super) fn found_rows(&self, snapshot: &ScreenSnapshot) -> Vec<Vec<(usize, usize, bool)>> {
        let find = &self.find;
        if !find.open || find.results.matches.is_empty() {
            return Vec::new();
        }
        // Lines in the results' coordinates: what was at `line` then is at `line - grown` now.
        let grown = snapshot.history_size.saturating_sub(find.history) as i32;
        let offset = snapshot.display_offset as i32;
        let cols = snapshot.columns();
        let row_of = |line: Line| usize::try_from(line.0 - grown + offset).ok();
        let cells = |line: Line| {
            let row = snapshot.rows().get(row_of(line)?)?;
            Some((row.cells(), row.extras()))
        };
        let mut rows = vec![Vec::new(); snapshot.rows().len()];
        for (row, found) in rows.iter_mut().enumerate() {
            let line = Line(row as i32 - offset + grown);
            for (ix, m) in find.results.on_line(line) {
                // Gone from under it (redrawn, or scrolled further than the history shows).
                if m.holds(cells, cols) == Some(false) {
                    continue;
                }
                if let Some((first, last)) = m.columns_on(line, cols) {
                    found.push((first, last, find.current == Some(ix)));
                }
            }
        }
        rows
    }

    /// Where the matches are along the scrollback, 0 at its top and 1 at the bottom of the
    /// screen: the ticks beside the terminal. Empty while find is closed.
    pub(super) fn found_ticks(&self, snapshot: &ScreenSnapshot) -> Vec<f32> {
        let find = &self.find;
        if !find.open {
            return Vec::new();
        }
        let grown = snapshot.history_size.saturating_sub(find.history) as i32;
        let top = snapshot.history_size as f32;
        let total = (snapshot.history_size + snapshot.rows().len()).max(1) as f32;
        find.results
            .matches
            .iter()
            .map(|m| ((m.start.line.0 - grown) as f32 + top) / total)
            .filter(|at| (0.0..=1.0).contains(at))
            .collect()
    }
}
