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
//!   says so. A plain one is working while a command that ends by itself runs in its foreground; the
//!   caller says so, since what stays open for ever (an editor, `ssh`, `less`, a dev server) would
//!   otherwise take every edit made outside the app. That makes an editor's own edits nobody's,
//!   like those of an editor outside the app. When both kinds were working, the told ones take the
//!   files: a build in another terminal never takes an agent's edits.
//! - Several terminals working at once all get the file (the honest answer: it can't be told), and
//!   nobody working means nobody: an edit from outside the app.
//! - A terminal that stopped working since the last snapshot still counts as having worked in
//!   between: the moment it stops is recorded, not only the samples that found it working, so its
//!   last edits are not lost between two ticks.
//! - A file stops being a terminal's when it leaves the status (committed, reverted).
//!
//! Pure, with no clock of its own: callers pass the `Instant`s, so the rules are tested without
//! waiting. It lives in memory; what the app keeps of it across restarts it hands back with
//! [`Attribution::restore`], and the first picture of a repository after that is a baseline like any
//! other: it attributes nothing new, but drops what has left the status meanwhile (a commit made
//! while the app was closed).

mod snapshot;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;

use snapshot::Prints;
pub use snapshot::Snapshot;

use crate::config::SharedChanges;

/// How a terminal says it is working.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// It tells its status: an agent's hooks, or a program's own report.
    Told,
    /// A shell, working while a command that ends by itself runs in its foreground.
    Plain,
}

/// Where a terminal is, as far as it is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place<'a> {
    /// Git has not answered for the folder it is in now (it just opened, moved to another window or
    /// changed folder): what was known of it stays, which is what keeps its count across a move.
    Unknown,
    /// In no repository.
    Outside,
    /// In the repository at this root, as git names it.
    In(&'a Path),
}

/// Where each terminal's changed files are counted, and the pictures that tell them.
#[derive(Debug, Default)]
pub struct Attribution {
    terminals: HashMap<i64, Terminal>,
    repos: HashMap<PathBuf, Repo>,
}

/// What is known of one terminal's activity.
#[derive(Debug)]
struct Terminal {
    /// The repository it is in (`None`: in none, or not known yet).
    root: Option<PathBuf>,
    kind: Kind,
    /// Working as of the last [`Attribution::track`].
    working: bool,
    /// The last time it was working, or stopped working: a `track` that found it so.
    last_working: Option<Instant>,
    /// Counts up whenever its set of files changes (not only how many): what the app compares to
    /// know that there is something new of this terminal to keep.
    revision: u64,
}

impl Terminal {
    fn new(kind: Kind) -> Self {
        Self { root: None, kind, working: false, last_working: None, revision: 0 }
    }

    fn edited(&mut self) {
        self.revision += 1;
    }
}

/// What is known of one repository.
#[derive(Debug, Default)]
struct Repo {
    /// The files as of the last picture; `None` before one (or after one with too many files):
    /// the next is a baseline.
    baseline: Option<Prints>,
    /// When the last picture was taken.
    taken: Option<Instant>,
    /// The paths each terminal changed that are still changed. Never an empty set.
    claims: BTreeMap<i64, HashSet<String>>,
}

impl Attribution {
    /// Say where terminal `id` is and whether it is working, as of `now`: from the window's tick, for
    /// every terminal. Returns whether a count changed, which only a terminal moving to another
    /// repository does.
    pub fn track(&mut self, id: i64, place: Place<'_>, kind: Kind, working: bool, now: Instant) -> bool {
        let terminal = self.terminals.entry(id).or_insert_with(|| Terminal::new(kind));
        terminal.kind = kind;
        // The moment it stops counts too: the tick that finds it idle is the last that knows it was
        // working until just before.
        if working || terminal.working {
            terminal.last_working = Some(now);
        }
        terminal.working = working;
        let root = match place {
            Place::Unknown => return false,
            Place::Outside => None,
            Place::In(root) => Some(root),
        };
        if terminal.root.as_deref() == root {
            return false;
        }
        // What it changed in the repository it left is not what its row counts any more.
        let left = std::mem::replace(&mut terminal.root, root.map(Path::to_path_buf));
        left.is_some_and(|left| self.leave(id, &left))
    }

    /// Terminal `id` is closed. Returns whether a count changed (its own, which goes).
    pub fn forget(&mut self, id: i64) -> bool {
        let Some(terminal) = self.terminals.remove(&id) else { return false };
        terminal.root.is_some_and(|root| self.leave(id, &root))
    }

    /// The files terminal `id` changed that are still uncommitted, if any, counted as `shared` says
    /// for a file that more than one terminal changed. The claims are kept as they are and the
    /// policy applies here, so changing it takes effect at once.
    pub fn changes_of(&self, id: i64, shared: SharedChanges) -> Option<usize> {
        let root = self.terminals.get(&id)?.root.as_deref()?;
        let repo = self.repos.get(root)?;
        let own = repo.claims.get(&id)?;
        let count = match shared {
            SharedChanges::Each => own.len(),
            SharedChanges::None => own
                .iter()
                .filter(|path| {
                    !repo.claims.iter().any(|(other, paths)| *other != id && paths.contains(*path))
                })
                .count(),
        };
        (count > 0).then_some(count)
    }

    /// The repository terminal `id` is in and the files it changed there, for keeping them.
    pub fn claims_of(&self, id: i64) -> Option<(&Path, &HashSet<String>)> {
        let root = self.terminals.get(&id)?.root.as_deref()?;
        Some((root, self.repos.get(root)?.claims.get(&id)?))
    }

    /// Terminal `id`'s revision: it moves when the set of its files does, so a revision that is not
    /// the one last kept means there is something new to keep. 0 for a terminal not known.
    pub fn revision_of(&self, id: i64) -> u64 {
        self.terminals.get(&id).map_or(0, |terminal| terminal.revision)
    }

    /// Put back what was kept of a terminal: the files it changed in the repository at `root`.
    /// Called as the terminal comes back, before anything is observed; the terminal is taken to be
    /// in that repository until [`track`](Self::track) says otherwise. Ignored for a terminal the
    /// model already knows (one that moved to another window in this run: what is in memory is newer
    /// than what was kept). Returns whether it has files again.
    pub fn restore(&mut self, id: i64, root: &Path, paths: impl IntoIterator<Item = String>) -> bool {
        if self.terminals.contains_key(&id) {
            return false;
        }
        let mut terminal = Terminal::new(Kind::Plain);
        terminal.root = Some(root.to_path_buf());
        self.terminals.insert(id, terminal);
        let paths: HashSet<String> = paths.into_iter().collect();
        let restored = !paths.is_empty();
        let repo = self.repos.entry(root.to_path_buf()).or_default();
        if restored {
            repo.claims.insert(id, paths);
        }
        restored
    }

    /// Take a new picture of a repository. Returns whether any terminal's count changed. A picture
    /// taken before the last one that came in arrived late and says nothing new.
    pub fn observe(&mut self, snapshot: Snapshot) -> bool {
        let Snapshot { root, taken, prints } = snapshot;
        let Self { terminals, repos } = self;
        let last = repos.get(&root).and_then(|repo| repo.taken);
        if last.is_some_and(|last| taken < last) {
            return false;
        }
        // Who was working since the picture before this one; none without one (a baseline).
        let claimants = last.map(|since| claimants(terminals, &root, since));
        let repo = repos.entry(root).or_default();
        repo.taken = Some(taken);
        let mut changed = false;
        match (prints, repo.baseline.take(), claimants) {
            // Too many files to say: forget what was known, the next picture starts over.
            (None, ..) => {
                changed = !repo.claims.is_empty();
                for id in repo.claims.keys() {
                    edited(terminals, *id);
                }
                repo.claims.clear();
            }
            (Some(now), Some(then), Some(claimants)) => {
                let touched: Vec<&str> = now.touched_since(&then).collect();
                if !touched.is_empty() {
                    for &id in &claimants {
                        repo.claims.entry(id).or_default();
                    }
                }
                repo.claims.retain(|id, paths| {
                    let before = paths.len();
                    let mut moved = false;
                    if claimants.contains(id) {
                        for path in &touched {
                            if !paths.contains(*path) {
                                paths.insert((*path).to_owned());
                                moved = true;
                            }
                        }
                    }
                    let grown = paths.len();
                    paths.retain(|path| now.has(path));
                    moved |= paths.len() != grown;
                    changed |= paths.len() != before;
                    if moved {
                        edited(terminals, *id);
                    }
                    !paths.is_empty()
                });
                repo.baseline = Some(now);
            }
            // The first picture: what is changed already is nobody's. What was restored is only kept
            // for the files still changed: a commit made while the app was closed takes the rest.
            (Some(now), ..) => {
                repo.claims.retain(|id, paths| {
                    let before = paths.len();
                    paths.retain(|path| now.has(path));
                    let pruned = paths.len() != before;
                    changed |= pruned;
                    if pruned {
                        edited(terminals, *id);
                    }
                    !paths.is_empty()
                });
                repo.baseline = Some(now);
            }
        }
        changed
    }

    /// Terminal `id` is no longer in `root`: its files there are dropped, and the repository is
    /// forgotten when no terminal is in it. Returns whether the terminal had any.
    fn leave(&mut self, id: i64, root: &Path) -> bool {
        let had = self.repos.get_mut(root).is_some_and(|repo| repo.claims.remove(&id).is_some());
        if had {
            edited(&mut self.terminals, id);
        }
        if !self.terminals.values().any(|terminal| terminal.root.as_deref() == Some(root)) {
            self.repos.remove(root);
        }
        had
    }
}

/// The terminals in `root` that take files touched since `since`: those that were working at any
/// moment since, or are now; the told ones alone when there are any.
fn claimants(terminals: &HashMap<i64, Terminal>, root: &Path, since: Instant) -> Vec<i64> {
    let working: Vec<(i64, Kind)> = terminals
        .iter()
        .filter(|(_, terminal)| terminal.root.as_deref() == Some(root))
        .filter(|(_, terminal)| terminal.working || terminal.last_working.is_some_and(|at| at >= since))
        .map(|(&id, terminal)| (id, terminal.kind))
        .collect();
    let told = working.iter().any(|&(_, kind)| kind == Kind::Told);
    let wanted = if told { Kind::Told } else { Kind::Plain };
    working.into_iter().filter(|&(_, kind)| kind == wanted).map(|(id, _)| id).collect()
}

/// Terminal `id`'s files changed (a terminal the model has dropped has nothing left to keep).
fn edited(terminals: &mut HashMap<i64, Terminal>, id: i64) {
    if let Some(terminal) = terminals.get_mut(&id) {
        terminal.edited();
    }
}
