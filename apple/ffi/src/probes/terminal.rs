//! A terminal's side of the Probes: opening and closing folds, the pointer revealing a shortened span, a rule's
//! file made real, and the guards that keep what needs the person from sitting under a fold.

use std::sync::atomic::Ordering;

use wormhole_core::probe::hidden::{reveal, shown};
use wormhole_core::probe::open_path::{self, PathRef};
use wormhole_term::RevealWhen;
use wormhole_text::keys;

use super::state;
use crate::terminal::Terminal;

/// A file a hook offered or a rule's link named, found in the terminal's folder.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ResolvedFile {
    /// The real path of a regular file, or `None` when it was not found (then `words` say so).
    pub path: Option<String>,
    /// 1-based.
    pub line: Option<u32>,
    /// The chip that says the file is not in this terminal's folder; the name is cleaned and cut.
    pub words: Option<String>,
}

#[uniffi::export]
impl Terminal {
    /// Whether any fold exists: Edit → Show Hidden Lines is greyed out without one.
    pub fn has_folds(&self) -> bool {
        self.with_pty(|pty| pty.has_folds()).unwrap_or(false)
    }

    /// Open or close fold `id` (the marker was clicked, or VoiceOver's Show). Whether it exists.
    pub fn fold_toggle(&self, id: u64) -> bool {
        self.with_pty(|pty| pty.fold_toggle(id)).unwrap_or(false)
    }

    /// Edit → Show Hidden Lines: open every fold of this terminal. How many lines that showed (0: none was closed).
    pub fn fold_show_all(&self) -> u32 {
        let hidden: usize = self.with_snapshot(|snapshot| {
            snapshot
                .rows()
                .iter()
                .filter_map(|row| row.fold())
                .filter(|fold| !fold.expanded)
                .map(|fold| fold.hidden())
                .sum()
        });
        let opened = self.with_pty(|pty| pty.fold_show_all()).unwrap_or(false);
        if opened { u32::try_from(hidden).unwrap_or(u32::MAX) } else { 0 }
    }

    /// What VoiceOver says when folds opened: "124 lines shown".
    pub fn fold_shown_words(&self, lines: u32) -> String {
        keys::terminal_fold_shown(wormhole_text::whole(lines))
    }

    /// The pointer is over viewport cell (`row`, `col`), with ⌥ as `alt`; `None` row and column: it left. A
    /// shortened or redacted span shows its text while the pointer is over it, when its rule allows. Whether the
    /// revealed span changed, so the view takes a new frame.
    pub fn set_reveal(&self, row: Option<u32>, col: Option<u32>, alt: bool) -> bool {
        let cell = row.zip(col).map(|(row, col)| (row, col, alt));
        let mut marks = self.marks.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if cell == marks.last_cell() && cell.is_some() {
            return false;
        }
        let at = cell.and_then(|(row, col, alt)| self.reveal_at(row, col, alt));
        marks.set_reveal(cell, at)
    }

    /// `text` (`path[:line[:column]]`) in this terminal's folder, through the hardened reads of `storage::files`
    /// (a path out of the folder, or through a link that leaves it, is not found). Reads the disk: off the main thread.
    pub fn resolve_path(&self, text: String) -> ResolvedFile {
        let found =
            PathRef::parse(&text).zip(self.with_pty(|pty| pty.cwd()).flatten()).and_then(|(target, cwd)| {
                match open_path::resolve(&[cwd.as_path()], &target) {
                    Ok(path) => Some((path, target.line)),
                    Err(err) => {
                        tracing::info!("a file a Probe named was not opened: {err}");
                        None
                    }
                }
            });
        match found {
            Some((path, line)) => {
                ResolvedFile { path: Some(path.to_string_lossy().into_owned()), line, words: None }
            }
            None => ResolvedFile {
                path: None,
                line: None,
                words: Some(keys::notice_failed_file_not_found(&shown(&text))),
            },
        }
    }
}

impl Terminal {
    /// The viewport cell where the masked span under (`row`, `col`) starts, when its rule lets it show.
    fn reveal_at(&self, row: u32, col: u32, alt: bool) -> Option<(usize, usize)> {
        let (rules, _) = state::rules();
        let rules = rules?;
        let point = self.grid_point(row, col);
        let span = self.with_pty(|pty| pty.rule_span_at(point, &*rules)).flatten()?;
        let allowed = match span.hit.reveal {
            RevealWhen::Hover => true,
            RevealWhen::Deliberate => alt,
            RevealWhen::Never => false,
        };
        let row = self.with_snapshot(|snapshot| snapshot.row_of_line(span.start.line))?;
        (span.hit.masked && allowed).then_some((row, span.start.column.0))
    }

    /// What the terminal being unable to hide says about itself: it needs the person (an agent's question, a
    /// program's report) or a program is reading a secret. Nothing the Probes fold may sit over either, whatever
    /// a forged status says: the rules of `wormhole_term::folds` hold regardless, these are their inputs.
    pub(crate) fn fold_needs_you(&self, needs_you: bool) {
        if self.needs_you.swap(needs_you, Ordering::Relaxed) == needs_you {
            return;
        }
        self.with_pty(|pty| {
            pty.set_fold_needs_you(needs_you);
            if needs_you {
                pty.fold_expand_recent();
            }
        });
    }

    /// Output came: while folds exist, a program reading a secret opens the recent ones and refuses new ones.
    pub(crate) fn fold_check_secret(&self) {
        self.with_pty(|pty| pty.has_folds() && pty.refresh_fold_secure());
    }

    /// A bell: what is hidden near the end of the output is shown, so the person can find what rang.
    pub(crate) fn fold_bell(&self) {
        self.with_pty(|pty| {
            if pty.has_folds() {
                pty.fold_expand_recent();
            }
        });
    }
}

/// Where a rule's link goes, for the chip: what the rule says of it, the target and the Probe. The target is always
/// shown: a rule's words must not stand in for where the link really goes.
pub(crate) fn readout(hover: Option<&str>, uri: &str, owner: &str) -> String {
    let name = state::probe_name(owner).unwrap_or_else(|| owner.to_string());
    let mut text = String::new();
    if let Some(hover) = hover {
        text.push_str(hover);
        text.push_str(" → ");
    }
    text.push_str(&reveal(uri));
    text.push_str(" · ");
    text.push_str(&reveal(&name));
    text
}
