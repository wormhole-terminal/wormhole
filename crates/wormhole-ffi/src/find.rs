//! Find in the whole scrollback, as the GPUI app does it (`terminal_view/find.rs`): `wormhole_term::find`
//! searches a copy of the grid's text, off the engine's lock; the results stay here, in grid
//! coordinates, and Swift asks for what it paints (matches on the viewport rows, ticks along the
//! history) and for the count and the current match. Matches stay right until the program prints:
//! the history growing since the search shifts them up, and a match whose cells no longer hold what
//! it matched is not drawn.
//!
//! Threads: `find_search` runs the whole search on the calling thread (call it off the main
//! thread); the lock is held only to copy the text. A search that finished after a newer one began
//! reports its outcome but does not replace the results.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use wormhole_term::find::{FindOptions, FindQuery, FindResults};
use wormhole_term::types::Line;

use crate::terminal::Terminal;

/// What the terminal keeps of the last search.
#[derive(Default)]
pub(crate) struct Finder {
    found: Mutex<Found>,
    /// Bumped by each search and by `find_clear`: a result from an older one is dropped.
    generation: AtomicU64,
}

#[derive(Default)]
struct Found {
    results: Arc<FindResults>,
    /// The history's length the results are as of.
    history: usize,
    /// The output revision the results are as of.
    revision: u64,
    current: Option<usize>,
}

/// What one search found.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, uniffi::Record)]
pub struct FindOutcome {
    pub count: u32,
    /// More than 10,000 matched; only the first are kept (the count reads "10,000+").
    pub capped: bool,
    /// The query is a regular expression that does not parse (or is too big).
    pub invalid: bool,
    /// The match nearest the bottom of the view, where a fresh search starts.
    pub nearest: Option<u32>,
}

/// A fold's marker as it reads while a search is open: the frame's words say nothing of matches.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FoldWords {
    pub row: u32,
    pub label: String,
    pub spoken: String,
}

/// A match on one viewport row of the last frame: its columns `first...=last`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct FoundSpan {
    pub row: u32,
    pub first: u32,
    pub last: u32,
    pub current: bool,
}

#[uniffi::export]
impl Terminal {
    /// Search the history and the screen. An empty query finds nothing; `regex` reads the text as a
    /// pattern, `whole_word` puts word boundaries, `match_case` is off by default (as the bar's).
    pub fn find_search(&self, text: String, match_case: bool, whole_word: bool, regex: bool) -> FindOutcome {
        let ticket = self.finder.generation.fetch_add(1, Ordering::AcqRel) + 1;
        let query = FindQuery { text, options: FindOptions { match_case, whole_word, regex } };
        let compiled = query.compile();
        let invalid = compiled.is_err();
        let searched = compiled.ok().flatten().and_then(|re| {
            let (grid, revision) = self.with_pty(|pty| pty.scrollback().grid_text())?;
            Some((grid.find(&re), grid.history(), revision))
        });
        let Some((results, history, revision)) = searched else {
            self.store(ticket, Found::default());
            return FindOutcome { invalid, ..FindOutcome::default() };
        };
        let nearest = self.with_snapshot(|snapshot| {
            let bottom = Line(snapshot.rows().len() as i32 - 1 - snapshot.display_offset as i32);
            results.nearest(bottom + grown(snapshot.history_size, history))
        });
        let outcome = FindOutcome {
            count: u32::try_from(results.matches.len()).unwrap_or(u32::MAX),
            capped: results.capped,
            invalid: false,
            nearest: nearest.map(|n| n as u32),
        };
        self.store(ticket, Found { results: Arc::new(results), history, revision, current: None });
        outcome
    }

    /// Make match `index` of the last search the current one (the one painted apart); `None` clears it.
    pub fn find_set_current(&self, index: Option<u32>) {
        let mut found = self.finder.found.lock().unwrap_or_else(PoisonError::into_inner);
        found.current = index.map(|i| i as usize).filter(|&i| i < found.results.matches.len());
    }

    /// Make match `index` current and scroll the view so it shows: a match out of view comes to the
    /// middle of the screen, one in view leaves the view where it is. The next `frame` is complete.
    pub fn find_reveal(&self, index: u32) {
        self.find_set_current(Some(index));
        let (results, history, current) = self.snapshot_of_found();
        let Some(found) = current.and_then(|i| results.matches.get(i)) else { return };
        let now = self.with_snapshot(|snapshot| snapshot.history_size);
        let line = found.start.line - grown(now, history);
        // A match inside a fold opens it, and stays open (as the GPUI app's `reveal_current` does).
        self.with_pty(|pty| {
            pty.fold_expand_at(line);
            pty.reveal(line);
        });
    }

    /// The markers of the last frame's rows with how many of the search's matches are inside each, for the rows that
    /// have one; empty with no match or no marker in view (read the frame first).
    pub fn find_fold_words(&self) -> Vec<FoldWords> {
        let (results, history, _) = self.snapshot_of_found();
        if results.matches.is_empty() {
            return Vec::new();
        }
        self.with_snapshot(|snapshot| {
            // What was at `line` now was at `line + grown` when searched.
            let grown = grown(snapshot.history_size, history);
            let matches = &results.matches;
            snapshot
                .rows()
                .iter()
                .enumerate()
                .filter_map(|(row, shown)| {
                    let fold = shown.fold()?;
                    let (first, last) = (fold.first + grown, fold.last + grown);
                    let inside = matches.partition_point(|m| m.start.line <= last)
                        - matches.partition_point(|m| m.start.line < first);
                    let words = crate::probes::words_of(fold, inside);
                    Some(FoldWords { row: row as u32, label: words.label, spoken: words.spoken })
                })
                .collect()
        })
    }

    /// The matches on each viewport row of the last frame, the current one flagged (read the frame first).
    /// A match whose cells changed since the search is left out.
    pub fn find_spans(&self) -> Vec<FoundSpan> {
        let (results, history, current) = self.snapshot_of_found();
        if results.matches.is_empty() {
            return Vec::new();
        }
        self.with_snapshot(|snapshot| {
            // What was at `line` when searched is at `line - grown` now.
            let grown = grown(snapshot.history_size, history);
            let offset = snapshot.display_offset as i32;
            let cols = snapshot.columns();
            let cells = |line: Line| {
                let row = snapshot.rows().get(usize::try_from(line.0 - grown + offset).ok()?)?;
                Some((row.cells(), row.extras()))
            };
            let mut spans = Vec::new();
            for row in 0..snapshot.rows().len() {
                let line = Line(row as i32 - offset + grown);
                for (index, found) in results.on_line(line) {
                    if found.holds(cells, cols) == Some(false) {
                        continue;
                    }
                    if let Some((first, last)) = found.columns_on(line, cols) {
                        spans.push(FoundSpan {
                            row: row as u32,
                            first: first as u32,
                            last: last as u32,
                            current: current == Some(index),
                        });
                    }
                }
            }
            spans
        })
    }

    /// Where the matches are along the scrollback, 0 at its top and 1 at the bottom of the screen, as of
    /// the last frame: the ticks beside the terminal.
    pub fn find_ticks(&self) -> Vec<f64> {
        let (results, history, _) = self.snapshot_of_found();
        self.with_snapshot(|snapshot| {
            let grown = grown(snapshot.history_size, history);
            let top = snapshot.history_size as f64;
            let total = (snapshot.history_size + snapshot.rows().len()).max(1) as f64;
            results
                .matches
                .iter()
                .map(|m| (f64::from(m.start.line.0 - grown) + top) / total)
                .filter(|at| (0.0..=1.0).contains(at))
                .collect()
        })
    }

    /// The program printed since the last search: its results may be stale.
    pub fn find_has_new_output(&self) -> bool {
        let revision = self.finder.found.lock().unwrap_or_else(PoisonError::into_inner).revision;
        self.with_pty(|pty| pty.output_revision() != revision).unwrap_or(false)
    }

    /// Forget the results (the bar closed) and drop any search still running.
    pub fn find_clear(&self) {
        self.finder.generation.fetch_add(1, Ordering::AcqRel);
        *self.finder.found.lock().unwrap_or_else(PoisonError::into_inner) = Found::default();
    }
}

impl Terminal {
    fn store(&self, ticket: u64, mut found: Found) {
        let mut slot = self.finder.found.lock().unwrap_or_else(PoisonError::into_inner);
        // Checked under the lock `find_clear` and the next search also take.
        if self.finder.generation.load(Ordering::Acquire) != ticket {
            return;
        }
        // A search again (the program printed) keeps the current match, clamped.
        let last = found.results.matches.len().checked_sub(1);
        found.current = slot.current.zip(last).map(|(current, last)| current.min(last));
        *slot = found;
    }

    fn snapshot_of_found(&self) -> (Arc<FindResults>, usize, Option<usize>) {
        let found = self.finder.found.lock().unwrap_or_else(PoisonError::into_inner);
        (found.results.clone(), found.history, found.current)
    }
}

/// How many lines the history grew since the search: the matches have moved up that far.
fn grown(history_now: usize, history_then: usize) -> i32 {
    history_now.saturating_sub(history_then) as i32
}
