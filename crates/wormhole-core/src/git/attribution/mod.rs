//! Which terminal changed which file.
//!
//! Git says what changed in a repository, not who changed it, and every terminal in a repository
//! asks the same question: counting `git status` in each row shows the same number everywhere. What
//! the app has to tell the terminals apart is when each was working. A hook carries no file names,
//! so this attributes by activity: a file touched (new in the status, or looking different on disk)
//! between two snapshots of the repository belongs to the terminals that were working in between.
//!
//! - The first snapshot of a repository is only a baseline: what was changed already belongs to
//!   nobody.
//! - A terminal whose status is told (an agent, or a program reporting its own) is working while it
//!   says so; a plain one while a command runs in its foreground. When both kinds were working, the
//!   told ones take the files: a dev server in another terminal never takes an agent's edits.
//! - Several terminals working at once all get the file (the honest answer: it can't be told), and
//!   nobody working means nobody: an editor outside the app made the change.
//! - A file stops being a terminal's when it leaves the status (committed, reverted).
//!
//! Pure, with no clock of its own: callers pass the `Instant`s, so the rules are tested without
//! waiting. It lives in memory only, so a restart of the app starts every count from nothing.

mod snapshot;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use snapshot::Prints;
pub use snapshot::{MAX_FILES, Snapshot};

/// A terminal's id, as the window names it.
pub type TerminalId = i64;

/// How a terminal says it is working.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// It tells its status: an agent's hooks, or a program's own report.
    Told,
    /// A shell, working while a command runs in its foreground.
    Plain,
}

/// Where each terminal's changed files are counted, and the pictures that tell them.
#[derive(Debug, Default)]
pub struct Attribution {
    terminals: HashMap<TerminalId, Terminal>,
    repos: HashMap<PathBuf, Repo>,
}

/// What is known of one terminal's activity.
#[derive(Debug)]
struct Terminal {
    /// The repository it is in, as git names it (`None`: in none).
    root: Option<PathBuf>,
    kind: Kind,
    /// Working as of the last [`Attribution::track`].
    working: bool,
    /// The last time a `track` found it working.
    last_working: Option<Instant>,
}

/// What is known of one repository.
#[derive(Debug, Default)]
struct Repo {
    /// The files as of the last snapshot; `None` before one (or after one with too many files):
    /// the next is a baseline.
    baseline: Option<Prints>,
    /// When the last snapshot was asked for, which is when its picture of the disk was taken.
    asked_at: Option<Instant>,
    /// The paths each terminal changed that are still changed. Never an empty set.
    claims: BTreeMap<TerminalId, HashSet<String>>,
}

impl Repo {
    /// How many files each terminal has, to tell whether an observation moved a count.
    fn counts(&self) -> Vec<(TerminalId, usize)> {
        self.claims.iter().map(|(&id, paths)| (id, paths.len())).collect()
    }
}

impl Attribution {
    /// Say where terminal `id` is and whether it is working, as of `now`: from the window's tick,
    /// for every terminal. `root` is the repository it is in (`None`: not in one). Returns whether
    /// a count changed, which only a terminal moving to another repository does.
    pub fn track(
        &mut self,
        id: TerminalId,
        root: Option<&Path>,
        kind: Kind,
        working: bool,
        now: Instant,
    ) -> bool {
        let terminal = self.terminals.entry(id).or_insert_with(|| Terminal {
            root: None,
            kind,
            working: false,
            last_working: None,
        });
        terminal.kind = kind;
        terminal.working = working;
        if working {
            terminal.last_working = Some(now);
        }
        if terminal.root.as_deref() == root {
            return false;
        }
        // What it changed in the repository it left is not what its row counts any more.
        let left = std::mem::replace(&mut terminal.root, root.map(Path::to_path_buf));
        left.is_some_and(|left| self.leave(id, &left))
    }

    /// Terminal `id` is closed. Returns whether a count changed (its own, which goes).
    pub fn forget(&mut self, id: TerminalId) -> bool {
        let Some(terminal) = self.terminals.remove(&id) else { return false };
        terminal.root.is_some_and(|root| self.leave(id, &root))
    }

    /// The files terminal `id` changed that are still uncommitted, if any.
    pub fn changes_of(&self, id: TerminalId) -> Option<usize> {
        let root = self.terminals.get(&id)?.root.as_deref()?;
        let count = self.repos.get(root)?.claims.get(&id)?.len();
        (count > 0).then_some(count)
    }

    /// Take a new picture of a repository, asked for at `asked_at`. Returns whether any terminal's
    /// count changed. A picture asked for before the last one taken arrived late and says nothing new.
    pub fn observe(&mut self, snapshot: Snapshot, asked_at: Instant) -> bool {
        let Snapshot { root, prints } = snapshot;
        let Self { terminals, repos } = self;
        let repo = repos.entry(root.clone()).or_default();
        if repo.asked_at.is_some_and(|last| asked_at < last) {
            return false;
        }
        let before = repo.counts();
        let previous = repo.asked_at.replace(asked_at);
        match (prints, repo.baseline.take(), previous) {
            // Too many files to say: forget what was known, the next picture starts over.
            (None, ..) => repo.claims.clear(),
            (Some(now), Some(then), Some(since)) => {
                let touched: Vec<&str> = now.touched_since(&then).collect();
                for id in claimants(terminals, &root, since) {
                    let paths = repo.claims.entry(id).or_default();
                    for path in &touched {
                        if !paths.contains(*path) {
                            paths.insert((*path).to_owned());
                        }
                    }
                }
                repo.claims.retain(|_, paths| {
                    paths.retain(|path| now.has(path));
                    !paths.is_empty()
                });
                repo.baseline = Some(now);
            }
            // The first picture: what is changed already is nobody's.
            (Some(now), ..) => repo.baseline = Some(now),
        }
        repo.counts() != before
    }

    /// Terminal `id` is no longer in `root`: its files there are dropped, and the repository is
    /// forgotten when no terminal is in it. Returns whether the terminal had any.
    fn leave(&mut self, id: TerminalId, root: &Path) -> bool {
        let had = self.repos.get_mut(root).is_some_and(|repo| repo.claims.remove(&id).is_some());
        let terminals = &self.terminals;
        self.repos.retain(|root, _| terminals.values().any(|terminal| terminal.root.as_ref() == Some(root)));
        had
    }
}

/// The terminals in `root` that take files touched since `since`: those that were working at any
/// moment since, or are now; the told ones alone when there are any.
fn claimants(terminals: &HashMap<TerminalId, Terminal>, root: &Path, since: Instant) -> Vec<TerminalId> {
    let working: Vec<(TerminalId, Kind)> = terminals
        .iter()
        .filter(|(_, terminal)| terminal.root.as_deref() == Some(root))
        .filter(|(_, terminal)| terminal.working || terminal.last_working.is_some_and(|at| at >= since))
        .map(|(&id, terminal)| (id, terminal.kind))
        .collect();
    let told = working.iter().any(|&(_, kind)| kind == Kind::Told);
    let wanted = if told { Kind::Told } else { Kind::Plain };
    working.into_iter().filter(|&(_, kind)| kind == wanted).map(|(id, _)| id).collect()
}
