//! The folder picker's providers and decorators: what plugs into `view::picker`.
//!
//! - [`Subfolders`]: the folders of the walked folder, matched forgivingly through several levels (`walk`).
//! - [`Recent`]: the folders the terminals were in lately, in their own section when nothing is typed and
//!   ranked in with the rest when something is.
//! - [`Chevrons`] and [`Branches`], decorators: whether a folder has folders in it, and the branch of a
//!   repository. They know a row's key is a path and nothing more, so they would decorate any provider's rows
//!   that are folders.
//!
//! They read the disk and block, so the app runs them off the main thread.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use super::insert::inserted;
use super::recent::RecentFolders;
use super::typed::{parse, tidy, up};
use super::walk::{Found, Hit, Limits, walk};
use crate::git::branch::{head_of, is_repository_top};
use crate::storage::files::{Cut, SCAN_CAP, has_folder};
use crate::view::picker::{Answer, Cancel, Crumb, Decoration, Decorator, Note, Provider, Query, Row, fit};

/// The SF Symbols the rows wear.
const FOLDER: &str = "folder";
/// What the rows are, for the decorators that know what a key of that kind is (a path).
pub const KIND: &str = "folder";
const CLOCK: &str = "clock.arrow.circlepath";

/// Most recent folders listed with nothing typed, and when something is.
const RECENT_IDLE: usize = 12;
const RECENT_TYPED: usize = 30;
/// The most recent folders looked at on the disk in one answer: a folder on a share that went away can make a
/// look wait, so there are few, and this provider answers on its own (the rows of the current folder never wait
/// for it).
const LOOKS: usize = 40;

/// The folders below the one the typed path walks to.
#[derive(Default)]
pub struct Subfolders {
    pub limits: Limits,
}

impl Provider for Subfolders {
    fn query(&self, query: &Query, cancel: &Cancel) -> Answer {
        let cx = &query.context;
        let typed = parse(&query.text, &cx.cwd, &cx.home);
        let found = walk(&typed, cancel, &self.limits);
        let mut notes = Vec::new();
        if found.failed {
            notes.push(Note::Failed);
        }
        match found.cut {
            Some(Cut::Kept) => notes.push(Note::Limited(self.limits.per_folder)),
            Some(Cut::Scanned) => notes.push(Note::Searched(SCAN_CAP)),
            None if found.limited => notes.push(Note::Limited(self.limits.rows)),
            None => {}
        }
        Answer {
            rows: found.hits.iter().filter_map(row).collect(),
            notes,
            up: up(&query.text, &cx.cwd, &cx.home),
            // A path ends at a level where it ends in a slash; a name still being typed does not.
            up_at_end: query.text.ends_with('/'),
            crumbs: crumbs(&found, &cx.home),
            ..Answer::default()
        }
    }
}

fn row(hit: &Hit) -> Option<Row> {
    Some(Row {
        marks: hit.marks.clone(),
        icon: FOLDER.into(),
        kind: KIND.into(),
        descend: Some(format!("{}{}/", hit.prefix, hit.label)),
        insert: Some(inserted(&hit.prefix, &hit.label)?),
        score: hit.score,
        ..Row::new(hit.path.to_string_lossy(), &hit.label)
    })
}

/// Where the person is, a step for each folder from home (`~`) or the root, each going back to that folder.
fn crumbs(found: &Found, home: &Path) -> Vec<Crumb> {
    path_crumbs(&found.base, home)
}

/// The steps of `path`: from `~` when it is under `home`, else from `/`.
pub fn path_crumbs(path: &Path, home: &Path) -> Vec<Crumb> {
    let below_home = (!home.as_os_str().is_empty() && home != Path::new("/"))
        .then(|| path.strip_prefix(home).ok())
        .flatten();
    let (mut text, rest, first) = match below_home {
        Some(rest) => ("~/".to_string(), rest, "~"),
        None => ("/".to_string(), path.strip_prefix("/").unwrap_or(path), "/"),
    };
    let mut out = vec![Crumb { label: first.into(), text: text.clone() }];
    for part in rest.components() {
        let name = part.as_os_str().to_string_lossy();
        text.push_str(&name);
        text.push('/');
        out.push(Crumb { label: name.into_owned(), text: text.clone() });
    }
    out
}

/// A path as it is typed: `~/…` under home, else whole.
pub fn display(path: &Path, home: &Path) -> String {
    if !home.as_os_str().is_empty() && home != Path::new("/") {
        if path == home {
            return "~".into();
        }
        if let Ok(rest) = path.strip_prefix(home) {
            return format!("~/{}", rest.display());
        }
    }
    path.display().to_string()
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The folders lately worked in.
pub struct Recent {
    recents: Arc<Mutex<RecentFolders>>,
    /// The section's title with nothing typed (the words are the app's).
    title: String,
    now: fn() -> u64,
}

impl Recent {
    /// `recents` is shared with whoever records and forgets, so a forgotten folder is gone from the next answer.
    pub fn new(recents: Arc<Mutex<RecentFolders>>, title: impl Into<String>) -> Self {
        Self { recents, title: title.into(), now }
    }

    /// With a clock of its own, for tests.
    pub fn at(mut self, now: fn() -> u64) -> Self {
        self.now = now;
        self
    }
}

/// Whether typed text is about a name, not a path being walked: `term` and `pr/te` are, `../`, `~/x`, `/usr` and
/// `src/` (a folder to go into) are not.
fn names_only(text: &str) -> bool {
    !(text.starts_with('/')
        || text.starts_with("~/")
        || text.starts_with("./")
        || text.starts_with("../")
        || matches!(text, "." | "..")
        || text.ends_with('/'))
}

impl Provider for Recent {
    fn query(&self, query: &Query, cancel: &Cancel) -> Answer {
        let cx = &query.context;
        let ranked: Vec<(String, f64)> = match self.recents.lock() {
            Ok(recents) => {
                recents.ranked((self.now)()).into_iter().map(|(p, s)| (p.to_string(), s)).collect()
            }
            Err(_) => Vec::new(),
        };
        let typed = query.text.as_str();
        let idle = typed.is_empty();
        if !idle && !names_only(typed) {
            return Answer::default();
        }
        let wanted: Vec<&str> = typed.split('/').filter(|s| !s.is_empty()).collect();
        let cap = if idle { RECENT_IDLE } else { RECENT_TYPED };
        let mut rows = Vec::new();
        let mut looks = 0;
        for (path, frecency) in ranked {
            if rows.len() >= cap || cancel.is_cancelled() {
                break;
            }
            let path = Path::new(&path);
            if idle && path == cx.cwd {
                continue;
            }
            let (score, marks) = if idle {
                (0, Vec::new())
            } else {
                match fit_tail(path, &wanted) {
                    Some((score, marks)) => (score + (frecency * 25.0).min(300.0) as i32, marks),
                    None => continue,
                }
            };
            // A folder that is gone is not offered (and costs one look).
            looks += 1;
            if looks > LOOKS {
                break;
            }
            if !path.is_dir() {
                continue;
            }
            if let Some(row) = recent_row(path, &cx.home, score, marks) {
                rows.push(row);
            }
        }
        if idle {
            rows.iter_mut().for_each(|r| r.section = 1);
            return Answer { sections: vec![String::new(), self.title.clone()], rows, ..Answer::default() };
        }
        Answer { rows, ..Answer::default() }
    }
}

/// How the last folders of `path` fit the steps typed (`wanted[0]` the folder furthest out, the last the
/// folder itself): the sum of how each fits, and where the last matched in the folder's name.
fn fit_tail(path: &Path, wanted: &[&str]) -> Option<(i32, Vec<std::ops::Range<usize>>)> {
    let names: Vec<String> = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(n) => Some(n.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    if wanted.is_empty() || names.len() < wanted.len() {
        return None;
    }
    let tail = &names[names.len() - wanted.len()..];
    let mut score = 0;
    let mut marks = Vec::new();
    for (name, step) in tail.iter().zip(wanted) {
        // A dot-folder answers only a step that starts with a dot.
        if name.starts_with('.') && !step.starts_with('.') {
            return None;
        }
        let f = fit(name, step)?;
        score += f.score;
        marks = f.marks;
    }
    Some((score, marks))
}

fn recent_row(path: &Path, home: &Path, score: i32, marks: Vec<std::ops::Range<usize>>) -> Option<Row> {
    let shown = display(path, home);
    let leaf = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let (label, insert, detail) = if shown == "~" {
        ("~".to_string(), "~/".to_string(), None)
    } else {
        let (Some(leaf), Some(parent)) = (leaf, path.parent()) else { return None };
        let mut prefix = display(parent, home);
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        let insert = inserted(&prefix, &leaf)?;
        (leaf, insert, Some(display(parent, home)))
    };
    Some(Row {
        marks,
        icon: CLOCK.into(),
        kind: KIND.into(),
        detail,
        descend: Some(format!("{shown}/")),
        insert: Some(insert),
        forgettable: true,
        score,
        ..Row::new(tidy(path).to_string_lossy(), label)
    })
}

/// Whether a folder has folders in it: the chevron.
pub struct Chevrons;

impl Decorator for Chevrons {
    fn kinds(&self) -> &[&'static str] {
        &[KIND]
    }

    fn decorate(&self, keys: &[String], cancel: &Cancel) -> Vec<Decoration> {
        let mut out = Vec::new();
        for key in keys {
            if cancel.is_cancelled() {
                break;
            }
            out.push(Decoration { key: key.clone(), chevron: Some(has_folder(Path::new(key))), badge: None });
        }
        out
    }
}

/// The branch of a folder that is a repository's top.
pub struct Branches;

impl Decorator for Branches {
    fn kinds(&self) -> &[&'static str] {
        &[KIND]
    }

    fn decorate(&self, keys: &[String], cancel: &Cancel) -> Vec<Decoration> {
        let mut out = Vec::new();
        for key in keys {
            if cancel.is_cancelled() {
                break;
            }
            let path = Path::new(key);
            if is_repository_top(path)
                && let Some(head) = head_of(path)
            {
                out.push(Decoration {
                    key: key.clone(),
                    badge: Some(head.label().to_string()),
                    chevron: None,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::{Arc, Mutex};

    use super::{Branches, Chevrons, Recent, Subfolders, display, path_crumbs};
    use crate::folder_picker::recent::RecentFolders;
    use crate::view::picker::{Answer, Cancel, Context, Decorator, Note, Provider, Query};

    fn query(text: &str, cwd: &Path, home: &Path) -> Query {
        Query {
            text: text.into(),
            context: Context { cwd: cwd.into(), home: home.into(), profile: "p".into() },
        }
    }

    fn make(dirs: &[&str]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for dir in dirs {
            fs::create_dir_all(root.path().join(dir)).unwrap();
        }
        root
    }

    fn labels(a: &Answer) -> Vec<&str> {
        a.rows.iter().map(|r| r.label.as_str()).collect()
    }

    #[test]
    fn subfolders_list_in_ls_order_and_each_row_says_what_it_inserts_and_descends_to() {
        let root = make(&["beta/inner", "Alpha", "with space", ".hidden"]);
        let home = Path::new("/nowhere");
        let a = Subfolders::default().query(&query("", root.path(), home), &Cancel::new());
        assert_eq!(labels(&a), ["Alpha", "beta", "with space"]);
        let space = &a.rows[2];
        assert_eq!(space.insert.as_deref(), Some(r"with\ space/"));
        assert_eq!(space.descend.as_deref(), Some("with space/"));
        assert_eq!(space.icon, "folder");
        assert_eq!(space.key, root.path().join("with space").to_string_lossy());
        assert_eq!(a.up.as_deref(), Some("../"));
        assert!(!a.up_at_end, "nothing typed: the text doesn't end at a level");
        assert!(a.rows.iter().all(|r| r.kind == "folder"), "the rows say what they are");
        assert_eq!(a.sections, [""]);
    }

    #[test]
    fn walking_down_keeps_the_typed_form_and_offers_the_way_up_and_crumbs() {
        let root = make(&["src/ui", "src/core"]);
        let home = root.path();
        let a = Subfolders::default().query(&query("~/src/", root.path(), home), &Cancel::new());
        assert_eq!(labels(&a), ["core", "ui"]);
        assert_eq!(a.rows[0].insert.as_deref(), Some("~/src/core/"));
        assert_eq!(a.rows[0].descend.as_deref(), Some("~/src/core/"));
        assert_eq!(a.up.as_deref(), Some("~/"));
        assert!(a.up_at_end, "a path that ends in a slash ends at a level");
        let crumbs: Vec<_> = a.crumbs.iter().map(|c| (c.label.as_str(), c.text.as_str())).collect();
        assert_eq!(crumbs, [("~", "~/"), ("src", "~/src/")]);
    }

    #[test]
    fn a_fuzzy_path_finds_the_folder_and_marks_it() {
        let root = make(&["Projects/terminal", "Projects/website"]);
        let a = Subfolders::default().query(&query("pr/te", root.path(), Path::new("/h")), &Cancel::new());
        assert_eq!(a.rows[0].label, "Projects/terminal");
        assert_eq!(a.rows[0].insert.as_deref(), Some("Projects/terminal/"));
        assert_eq!(a.rows[0].marks, [0..2, 9..11]);
    }

    #[test]
    fn a_name_with_a_dash_first_is_guarded_and_a_folder_that_cant_be_read_is_said() {
        let root = make(&["-rf"]);
        let a = Subfolders::default().query(&query("", root.path(), Path::new("/h")), &Cancel::new());
        assert_eq!(a.rows[0].insert.as_deref(), Some("./-rf/"));
        let gone = Subfolders::default()
            .query(&query("", &root.path().join("gone"), Path::new("/h")), &Cancel::new());
        assert!(gone.rows.is_empty());
        assert_eq!(gone.notes, [Note::Failed]);
    }

    #[test]
    fn crumbs_start_at_home_or_the_root() {
        let c = path_crumbs(Path::new("/Users/me/Projects"), Path::new("/Users/me"));
        let t: Vec<_> = c.iter().map(|c| c.text.as_str()).collect();
        assert_eq!(t, ["~/", "~/Projects/"]);
        let c = path_crumbs(Path::new("/usr/local"), Path::new("/Users/me"));
        let t: Vec<_> = c.iter().map(|c| (c.label.as_str(), c.text.as_str())).collect();
        assert_eq!(t, [("/", "/"), ("usr", "/usr/"), ("local", "/usr/local/")]);
        assert_eq!(path_crumbs(Path::new("/Users/me"), Path::new("/Users/me"))[0].label, "~");
        assert_eq!(display(Path::new("/Users/me/x"), Path::new("/Users/me")), "~/x");
        assert_eq!(display(Path::new("/usr"), Path::new("/Users/me")), "/usr");
    }

    fn recents(paths: &[&Path]) -> Arc<Mutex<RecentFolders>> {
        let mut r = RecentFolders::default();
        for (i, p) in paths.iter().enumerate() {
            r.record(p, 1_000 + 100 * i as u64, &[]);
        }
        Arc::new(Mutex::new(r))
    }

    #[test]
    fn recent_folders_have_a_section_when_nothing_is_typed_and_leave_out_the_current_one_and_gone_ones() {
        let root = make(&["a", "b/deep", "cwd"]);
        let gone = root.path().join("gone");
        let list =
            recents(&[&root.path().join("a"), &root.path().join("b/deep"), &gone, &root.path().join("cwd")]);
        let provider = Recent::new(list, "Recent").at(|| 2_000);
        let a = provider.query(&query("", &root.path().join("cwd"), root.path()), &Cancel::new());
        assert_eq!(a.sections, ["", "Recent"]);
        assert_eq!(labels(&a), ["deep", "a"], "newest first; the current and the gone folder left out");
        assert!(a.rows.iter().all(|r| r.section == 1 && r.forgettable && r.icon == "clock.arrow.circlepath"));
        assert_eq!(a.rows[0].insert.as_deref(), Some("~/b/deep/"), "under home it is ~/…");
        assert_eq!(a.rows[0].detail.as_deref(), Some("~/b"));
    }

    #[test]
    fn recent_folders_are_ranked_in_when_text_is_typed_and_not_when_a_path_is() {
        let root = make(&["Projects/terminal", "Projects/website"]);
        let list = recents(&[&root.path().join("Projects/terminal"), &root.path().join("Projects/website")]);
        let provider = Recent::new(list, "Recent").at(|| 2_000);
        let q = |t: &str| query(t, Path::new("/elsewhere"), root.path());
        let a = provider.query(&q("term"), &Cancel::new());
        assert_eq!(labels(&a), ["terminal"]);
        assert_eq!(a.sections, [""], "ranked in, no section of its own");
        assert_eq!(a.rows[0].marks.len(), 1);
        assert_eq!(a.rows[0].marks[0], 0..4);
        assert_eq!(a.rows[0].insert.as_deref(), Some("~/Projects/terminal/"));
        assert_eq!(
            labels(&provider.query(&q("pr/we"), &Cancel::new())),
            ["website"],
            "each step fits its folder"
        );
        for path_like in ["../x", "/usr", "~/x", "src/", ".."] {
            assert!(provider.query(&q(path_like), &Cancel::new()).rows.is_empty(), "{path_like}");
        }
    }

    #[test]
    fn forgetting_through_the_shared_list_removes_the_row() {
        let root = make(&["a"]);
        let list = recents(&[&root.path().join("a")]);
        let provider = Recent::new(list.clone(), "Recent").at(|| 2_000);
        let q = query("", Path::new("/elsewhere"), Path::new("/h"));
        assert_eq!(provider.query(&q, &Cancel::new()).rows.len(), 1);
        list.lock().unwrap().forget(&root.path().join("a").to_string_lossy());
        assert!(provider.query(&q, &Cancel::new()).rows.is_empty());
    }

    #[test]
    fn decorators_tell_chevrons_and_branches() {
        let root = make(&["plain", "has/child", "repo/.git", "repo/sub"]);
        fs::write(root.path().join("repo/.git/HEAD"), "ref: refs/heads/trunk\n").unwrap();
        let key = |n: &str| root.path().join(n).to_string_lossy().into_owned();
        let keys: Vec<String> = ["plain", "has", "repo"].iter().map(|n| key(n)).collect();
        let chevrons = Chevrons.decorate(&keys, &Cancel::new());
        let flags: Vec<_> = chevrons.iter().map(|d| d.chevron).collect();
        assert_eq!(flags, [Some(false), Some(true), Some(true)]);
        let branches = Branches.decorate(&keys, &Cancel::new());
        assert_eq!(branches.len(), 1);
        assert_eq!(
            (branches[0].key.as_str(), branches[0].badge.as_deref()),
            (key("repo").as_str(), Some("trunk"))
        );
        let cancelled = Cancel::new();
        cancelled.cancel();
        assert!(Chevrons.decorate(&keys, &cancelled).is_empty());
    }
}
