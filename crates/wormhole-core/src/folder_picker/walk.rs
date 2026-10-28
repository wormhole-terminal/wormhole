//! Finding folders for a typed path, forgivingly and through several levels: `pr/te` finds
//! `Projects/terminal` when `pr` fits a folder in the starting folder and `te` fits one inside it.
//!
//! A step that names a folder that is there exactly (`src` when there is a `src`) is taken exactly and goes
//! no further, so a path typed in full behaves as it always did. A step that names none is matched against the
//! folders of every place the steps before it reached, and only the best few of those are gone through (the
//! beam). All of it is bounded, because the disk is not ours: at most [`Limits::depth`] steps are matched
//! forgivingly, [`Limits::beam`] places are carried from one step to the next, [`Limits::reads`] folders are read
//! in all and at most [`Limits::rows`] folders come back. The reader checks [`Cancel`] before each read.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::path::{Path, PathBuf};

use super::collate::collate;
use super::typed::{Typed, tidy};
use crate::storage::files::{Cut, FOLDERS_CAP, list_folders_where};
use crate::view::picker::{Cancel, fit};

/// How far a walk may go.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The most steps matched forgivingly: a longer path needs its first steps to be exact.
    pub depth: usize,
    /// The most places carried from one step to the next.
    pub beam: usize,
    /// The most folders read in one walk.
    pub reads: usize,
    /// The most folders returned.
    pub rows: usize,
    /// The most subfolders kept from one folder.
    pub per_folder: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self { depth: 4, beam: 6, reads: 48, rows: 500, per_folder: FOLDERS_CAP }
    }
}

/// A folder found: where it is, how it would be typed and how well it fits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    pub path: PathBuf,
    /// The leading steps that were typed exactly, as typed, with the anchor and a closing slash: what a path
    /// to this folder starts with, so it stays relative, `~` or absolute as it was written.
    pub prefix: String,
    /// The rest of the way: the names after the exact steps, joined with `/`.
    pub label: String,
    /// Where the typed text matched in `label`, as byte ranges.
    pub marks: Vec<Range<usize>>,
    pub score: i32,
}

/// What a walk came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Found {
    pub hits: Vec<Hit>,
    /// The folder the exact steps reach, `.` and `..` worked out by the text alone: where the person is.
    pub base: PathBuf,
    /// The exact steps as typed with the anchor, the text that leads to `base`.
    pub base_text: String,
    /// The folder the typed path names exactly could not be read.
    pub failed: bool,
    /// A read stopped at a cap, or more folders matched than are returned.
    pub cut: Option<Cut>,
    pub limited: bool,
}

/// One step taken: the name, and whether it was typed exactly, and where the text matched in it.
#[derive(Clone, Debug)]
struct Step {
    name: String,
    exact: bool,
    marks: Vec<Range<usize>>,
}

/// A place reached, how, and how well.
#[derive(Clone, Debug)]
struct Place {
    path: PathBuf,
    steps: Vec<Step>,
    score: i32,
}

/// A step that is a move and not a name to match.
fn is_move(step: &str) -> bool {
    matches!(step, "" | "." | "..")
}

/// A dot-folder only answers a step that starts with a dot.
fn shown(name: &str, step: &str) -> bool {
    step.starts_with('.') || !name.starts_with('.')
}

/// A folder that fits a step.
struct Named {
    name: String,
    score: i32,
    marks: Vec<Range<usize>>,
}

/// The folders of `dir` that fit `step`, best first, ties in the order `ls` lists them; and why the read
/// stopped early, if it did.
fn fitting(dir: &Path, step: &str, cap: usize) -> std::io::Result<(Vec<Named>, Option<Cut>)> {
    // The read keeps what fits; how it fits is worked out once, there.
    let fits = RefCell::new(HashMap::new());
    let read = list_folders_where(dir, cap, |name| {
        if !shown(name, step) {
            return false;
        }
        fit(name, step).is_some_and(|found| fits.borrow_mut().insert(name.to_string(), found).is_none())
    })?;
    let mut fits = fits.into_inner();
    let mut names = read.names;
    names.sort_by(|a, b| collate(a, b));
    let mut found: Vec<Named> = names
        .into_iter()
        .filter_map(|name| {
            let f = fits.remove(&name)?;
            Some(Named { name, score: f.score, marks: f.marks })
        })
        .collect();
    found.sort_by_key(|named| std::cmp::Reverse(named.score));
    Ok((found, read.cut))
}

/// Walk `typed`: the folders it names, forgivingly, from where it starts.
pub fn walk(typed: &Typed, cancel: &Cancel, limits: &Limits) -> Found {
    let mut reads = 0;
    let forgiving = typed.steps.len() <= limits.depth;
    let mut places = vec![Place { path: typed.start.clone(), steps: Vec::new(), score: 0 }];
    for step in typed.through() {
        let mut next = Vec::new();
        for place in &places {
            if cancel.is_cancelled() {
                return Found::default();
            }
            if is_move(step) {
                let mut steps = place.steps.clone();
                if !step.is_empty() {
                    steps.push(Step { name: step.clone(), exact: true, marks: Vec::new() });
                }
                next.push(Place { path: place.path.join(step), steps, score: place.score });
                continue;
            }
            let exact = place.path.join(step);
            if exact.is_dir() {
                let mut steps = place.steps.clone();
                steps.push(Step { name: step.clone(), exact: true, marks: Vec::new() });
                next.push(Place { path: exact, steps, score: place.score + 1000 });
            } else if forgiving && reads < limits.reads {
                reads += 1;
                if let Ok((found, _)) = fitting(&place.path, step, limits.per_folder) {
                    for Named { name, score, marks } in found.into_iter().take(limits.beam) {
                        let mut steps = place.steps.clone();
                        steps.push(Step { name: name.clone(), exact: false, marks });
                        next.push(Place { path: place.path.join(name), steps, score: place.score + score });
                    }
                }
            }
        }
        next.sort_by_key(|place| std::cmp::Reverse(place.score));
        next.truncate(limits.beam);
        places = next;
        if places.is_empty() {
            return Found { base: tidy(&typed.start), ..Found::default() };
        }
    }

    let (base, base_text) = base_of(typed, &places);
    let last = typed.last();
    let single = places.len() == 1 && places[0].steps.iter().all(|s| s.exact);
    let mut found = Found { base, base_text, ..Found::default() };
    let mut hits = Vec::new();
    for place in &places {
        if cancel.is_cancelled() || reads >= limits.reads {
            break;
        }
        reads += 1;
        match fitting(&place.path, last, limits.per_folder) {
            Ok((names, cut)) => {
                found.cut = found.cut.or(cut);
                for Named { name, score, marks } in names {
                    let mut steps = place.steps.clone();
                    steps.push(Step { name: name.clone(), exact: false, marks });
                    hits.push(hit(typed, &place.path.join(&name), &steps, place.score + score));
                }
            }
            Err(_) => found.failed |= single,
        }
    }
    hits.sort_by_key(|hit| std::cmp::Reverse(hit.score));
    if hits.len() > limits.rows {
        hits.truncate(limits.rows);
        found.limited = true;
    }
    found.hits = hits;
    found
}

/// A hit for the folder at `path`, reached by `steps`: the exact steps at the start are the prefix as typed.
fn hit(typed: &Typed, path: &Path, steps: &[Step], score: i32) -> Hit {
    let exact = steps.iter().take_while(|s| s.exact).count();
    let mut prefix = typed.anchor.to_string();
    for step in &steps[..exact] {
        prefix.push_str(&step.name);
        prefix.push('/');
    }
    let mut label = String::new();
    let mut marks = Vec::new();
    for step in &steps[exact..] {
        if !label.is_empty() {
            label.push('/');
        }
        marks.extend(step.marks.iter().map(|m| label.len() + m.start..label.len() + m.end));
        label.push_str(&step.name);
    }
    Hit { path: tidy(path), prefix, label, marks, score }
}

/// Where the exact steps lead, which every place agrees on: the folder and the text that spells it.
fn base_of(typed: &Typed, places: &[Place]) -> (PathBuf, String) {
    let exact = places.first().map_or(0, |p| p.steps.iter().take_while(|s| s.exact).count());
    let mut path = typed.start.clone();
    let mut text = typed.anchor.to_string();
    if let Some(place) = places.first() {
        for step in &place.steps[..exact] {
            path.push(&step.name);
            text.push_str(&step.name);
            text.push('/');
        }
    }
    (tidy(&path), text)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{Limits, walk};
    use crate::folder_picker::typed::parse;
    use crate::view::picker::Cancel;

    fn tree(dirs: &[&str]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        for dir in dirs {
            fs::create_dir_all(root.path().join(dir)).unwrap();
        }
        root
    }

    fn labels(root: &Path, typed: &str) -> Vec<String> {
        let found = walk(&parse(typed, root, Path::new("/nowhere")), &Cancel::new(), &Limits::default());
        found.hits.iter().map(|h| format!("{}{}", h.prefix, h.label)).collect()
    }

    #[test]
    fn one_step_matches_the_folders_of_the_start_forgivingly_best_first() {
        let root = tree(&["Projects", "docs", "my-projects", ".git", "src"]);
        assert_eq!(
            labels(root.path(), ""),
            ["docs", "my-projects", "Projects", "src"],
            "ls order, no dot-folders"
        );
        assert_eq!(labels(root.path(), "pro"), ["Projects", "my-projects"]);
        assert_eq!(labels(root.path(), "prjcts"), ["Projects", "my-projects"], "letters in order");
        assert_eq!(labels(root.path(), ".g"), [".git"]);
        assert!(labels(root.path(), "zzz").is_empty());
    }

    #[test]
    fn steps_typed_exactly_are_gone_through_and_stay_as_typed() {
        let root = tree(&["src/ui/button", "src/core", "other"]);
        let found = walk(&parse("src/ui/", root.path(), Path::new("/h")), &Cancel::new(), &Limits::default());
        assert_eq!(found.hits.len(), 1);
        assert_eq!((found.hits[0].prefix.as_str(), found.hits[0].label.as_str()), ("src/ui/", "button"));
        assert_eq!(found.base, root.path().join("src/ui"));
        assert_eq!(found.base_text, "src/ui/");
        assert_eq!(labels(root.path(), "src/c"), ["src/core"]);
    }

    #[test]
    fn each_step_fits_a_folder_at_its_depth() {
        let root = tree(&["Projects/terminal/src", "Projects/website", "Pictures/trips", "terminal"]);
        assert_eq!(labels(root.path(), "pr/te")[0], "Projects/terminal", "the best fit leads");
        let found = walk(&parse("pr/te", root.path(), Path::new("/h")), &Cancel::new(), &Limits::default());
        assert_eq!(found.hits[0].label, "Projects/terminal");
        assert_eq!(found.hits[0].marks, [0..2, 9..11], "the letters that matched, in each name");
        assert_eq!(found.hits[0].prefix, "");
        assert_eq!(found.hits[0].path, root.path().join("Projects/terminal"));
    }

    #[test]
    fn a_fuzzy_step_then_a_slash_lists_inside_every_place_it_reached() {
        let root = tree(&["Projects/terminal", "Projects/website", "Pictures/trips"]);
        let all = labels(root.path(), "pr/");
        assert_eq!(
            all,
            ["Projects/terminal", "Projects/website", "Pictures/trips"],
            "the better place first"
        );
    }

    #[test]
    fn an_exact_step_after_a_fuzzy_one_is_part_of_the_label() {
        let root = tree(&["Projects/src", "Pictures/src"]);
        assert_eq!(labels(root.path(), "pro/src/"), Vec::<String>::new(), "nothing inside src");
        assert_eq!(labels(root.path(), "pro/s"), ["Projects/src"]);
    }

    #[test]
    fn dots_walk_by_the_text_and_the_base_is_where_the_person_is() {
        let root = tree(&["a/b", "a/c", "x"]);
        let at = root.path().join("a/b");
        let found = walk(&parse("../", &at, Path::new("/h")), &Cancel::new(), &Limits::default());
        let names: Vec<_> = found.hits.iter().map(|h| h.label.as_str()).collect();
        assert_eq!(names, ["b", "c"]);
        assert_eq!(found.hits[0].prefix, "../");
        assert_eq!(found.base, root.path().join("a"));
        assert_eq!(found.hits[0].path, root.path().join("a/b"), "a path with the dots worked out");
    }

    #[test]
    fn a_folder_that_cant_be_read_is_said_and_a_missing_start_has_no_hits() {
        let root = tree(&["a"]);
        let found = walk(&parse("nope/", root.path(), Path::new("/h")), &Cancel::new(), &Limits::default());
        assert!(found.hits.is_empty());
        assert!(!found.failed, "no such step even forgivingly is just an empty list");
        let found = walk(
            &parse("", &root.path().join("missing"), Path::new("/h")),
            &Cancel::new(),
            &Limits::default(),
        );
        assert!(found.failed, "the folder typed exactly is not readable");
    }

    #[test]
    fn the_walk_is_bounded_in_depth_reads_and_rows() {
        let root = tree(&["a1/b1/c1/d1/e1", "a2/b2", "a3", "a4", "a5", "a6", "a7", "a8"]);
        // Five steps: only an exact path goes through; a fuzzy first step finds nothing.
        assert!(labels(root.path(), "a/b/c/d/e").is_empty());
        assert_eq!(labels(root.path(), "a1/b1/c1/d1/e"), ["a1/b1/c1/d1/e1"]);
        let few = Limits { rows: 3, ..Limits::default() };
        let found = walk(&parse("a", root.path(), Path::new("/h")), &Cancel::new(), &few);
        assert_eq!(found.hits.len(), 3);
        assert!(found.limited);
        let one_read = Limits { reads: 1, ..Limits::default() };
        assert!(walk(&parse("a/b", root.path(), Path::new("/h")), &Cancel::new(), &one_read).hits.is_empty());
    }

    #[test]
    fn a_cancelled_walk_returns_nothing() {
        let root = tree(&["a/b"]);
        let cancel = Cancel::new();
        cancel.cancel();
        let found = walk(&parse("a/", root.path(), Path::new("/h")), &cancel, &Limits::default());
        assert!(found.hits.is_empty());
    }
}
