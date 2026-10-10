//! Three-way merge of `notes.md` with the app's notes, for sync conflicts.

use std::collections::{BTreeSet, HashMap};

use super::{NotesDoc, TaskItem, normalize};

/// Identity of an item in the merged output: derived from a base item, or added by one side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Key {
    Base(usize),
    Ours(usize),
    Theirs(usize),
}

/// Three-way merge for sync conflicts (e.g. the app edited notes while `git pull` changed the file).
///
/// Items are matched by content. Tasks: additions from both sides are kept (once if identical);
/// a checkbox change wins over no change; deleting a task on one side removes it unless the other
/// side changed it. Notes: a note missing on one side that closely resembles a note added on that
/// side (most words shared) counts as an edit; an edit wins over no change and over a deletion;
/// if both sides edited the same note differently, both versions are kept. Order follows `ours`,
/// with `theirs`' additions inserted after their preceding item.
pub fn merge_notes(base: &NotesDoc, ours: &NotesDoc, theirs: &NotesDoc) -> NotesDoc {
    let (base, ours, theirs) = (normalize(base), normalize(ours), normalize(theirs));
    NotesDoc {
        tasks: merge_tasks(&base.tasks, &ours.tasks, &theirs.tasks),
        notes: merge_note_texts(&base.notes, &ours.notes, &theirs.notes),
    }
}

fn merge_tasks(base: &[TaskItem], ours: &[TaskItem], theirs: &[TaskItem]) -> Vec<TaskItem> {
    let text = |t: &TaskItem| t.text.clone();
    let o_match = match_exact(base, ours, text);
    let t_match = match_exact(base, theirs, text);
    let (o_keys, t_keys) = side_keys(ours, theirs, &o_match, &t_match, text);

    let mut values: HashMap<Key, Vec<TaskItem>> = HashMap::new();
    for (bi, b) in base.iter().enumerate() {
        let o = o_match.iter().position(|m| *m == Some(bi)).map(|j| &ours[j]);
        let t = t_match.iter().position(|m| *m == Some(bi)).map(|k| &theirs[k]);
        let merged = match (o, t) {
            (Some(o), Some(t)) => Some(if o.done != b.done { o } else { t }),
            (Some(changed), None) | (None, Some(changed)) => (changed.done != b.done).then_some(changed),
            (None, None) => None,
        };
        if let Some(item) = merged {
            values.insert(Key::Base(bi), vec![item.clone()]);
        }
    }
    for (j, item) in ours.iter().enumerate() {
        if o_match[j].is_none() {
            // Added on both sides: done if either side finished it.
            let done_theirs = t_keys.iter().zip(theirs).any(|(k, t)| *k == Key::Ours(j) && t.done);
            values.insert(Key::Ours(j), vec![TaskItem { done: item.done || done_theirs, ..item.clone() }]);
        }
    }
    for (k, item) in theirs.iter().enumerate() {
        if t_keys[k] == Key::Theirs(k) {
            values.insert(Key::Theirs(k), vec![item.clone()]);
        }
    }
    emit(&o_keys, &t_keys, values)
}

fn merge_note_texts(base: &[String], ours: &[String], theirs: &[String]) -> Vec<String> {
    let text = |n: &String| n.clone();
    let o_match = with_edits(base, ours, match_exact(base, ours, text));
    let t_match = with_edits(base, theirs, match_exact(base, theirs, text));
    let (o_keys, t_keys) = side_keys(ours, theirs, &o_match, &t_match, text);

    let mut values: HashMap<Key, Vec<String>> = HashMap::new();
    for (bi, b) in base.iter().enumerate() {
        // None = deleted on that side; Some(text) = kept (possibly edited).
        let o = o_match.iter().position(|m| *m == Some(bi)).map(|j| &ours[j]);
        let t = t_match.iter().position(|m| *m == Some(bi)).map(|k| &theirs[k]);
        let o_edit = o.filter(|o| *o != b);
        let t_edit = t.filter(|t| *t != b);
        let merged: Vec<String> = match (o_edit, t_edit) {
            (Some(o), Some(t)) if o == t => vec![o.clone()],
            (Some(o), Some(t)) => vec![o.clone(), t.clone()],
            (Some(edit), None) | (None, Some(edit)) => vec![edit.clone()],
            // Unchanged on both sides, or unchanged on one and deleted on the other.
            (None, None) if o.is_some() && t.is_some() => vec![b.clone()],
            (None, None) => Vec::new(),
        };
        if !merged.is_empty() {
            values.insert(Key::Base(bi), merged);
        }
    }
    for (j, note) in ours.iter().enumerate() {
        if o_match[j].is_none() {
            values.insert(Key::Ours(j), vec![note.clone()]);
        }
    }
    for (k, note) in theirs.iter().enumerate() {
        if t_keys[k] == Key::Theirs(k) {
            values.insert(Key::Theirs(k), vec![note.clone()]);
        }
    }
    emit(&o_keys, &t_keys, values)
}

/// For each `side` item, the base index with equal content (duplicates matched in order).
fn match_exact<T>(base: &[T], side: &[T], key: impl Fn(&T) -> String) -> Vec<Option<usize>> {
    let mut used = BTreeSet::new();
    side.iter()
        .map(|item| {
            let k = key(item);
            let found = base.iter().enumerate().position(|(i, b)| !used.contains(&i) && key(b) == k);
            if let Some(i) = found {
                used.insert(i);
            }
            found
        })
        .collect()
}

/// Pair base notes missing on a side with similar notes added on that side, so they count as
/// edits instead of delete + add.
fn with_edits(base: &[String], side: &[String], mut matched: Vec<Option<usize>>) -> Vec<Option<usize>> {
    for (bi, b) in base.iter().enumerate() {
        if matched.contains(&Some(bi)) {
            continue;
        }
        let best = (0..side.len())
            .filter(|j| matched[*j].is_none())
            .map(|j| (j, similarity(b, &side[j])))
            .filter(|(_, score)| *score > 0.5)
            .fold(None, |best: Option<(usize, f64)>, (j, s)| match best {
                Some((_, bs)) if bs >= s => best,
                _ => Some((j, s)),
            });
        if let Some((j, _)) = best {
            matched[j] = Some(bi);
        }
    }
    matched
}

/// Share of the smaller note's words found in the other: appending to or lightly rewording a note
/// keeps it high, unrelated notes stay low.
fn similarity(a: &str, b: &str) -> f64 {
    let words = |s: &str| -> BTreeSet<String> {
        s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_lowercase).collect()
    };
    let (a, b) = (words(a), words(b));
    let smaller = a.len().min(b.len());
    if smaller == 0 {
        return 0.0;
    }
    a.intersection(&b).count() as f64 / smaller as f64
}

/// Keys of both sides. An item added on both sides with the same content gets the ours key.
fn side_keys<T>(
    ours: &[T],
    theirs: &[T],
    o_match: &[Option<usize>],
    t_match: &[Option<usize>],
    key: impl Fn(&T) -> String,
) -> (Vec<Key>, Vec<Key>) {
    let o_keys: Vec<Key> =
        o_match.iter().enumerate().map(|(j, m)| m.map_or(Key::Ours(j), Key::Base)).collect();
    let mut used = BTreeSet::new();
    let t_keys = t_match
        .iter()
        .enumerate()
        .map(|(k, m)| {
            if let Some(bi) = m {
                return Key::Base(*bi);
            }
            let same = (0..ours.len())
                .find(|j| o_match[*j].is_none() && !used.contains(j) && key(&ours[*j]) == key(&theirs[k]));
            match same {
                Some(j) => {
                    used.insert(j);
                    Key::Ours(j)
                }
                None => Key::Theirs(k),
            }
        })
        .collect();
    (o_keys, t_keys)
}

/// Lay out kept keys in `ours` order, inserting keys only `theirs` has after their predecessor
/// in `theirs` (or at the front if they had none), then expand to values.
fn emit<T>(ours: &[Key], theirs: &[Key], mut values: HashMap<Key, Vec<T>>) -> Vec<T> {
    let mut order: Vec<Key> = Vec::new();
    for key in ours {
        if values.contains_key(key) && !order.contains(key) {
            order.push(*key);
        }
    }
    for (ix, key) in theirs.iter().enumerate() {
        if !values.contains_key(key) || order.contains(key) {
            continue;
        }
        let pos = theirs[..ix]
            .iter()
            .rev()
            .find_map(|prev| order.iter().position(|o| o == prev))
            .map_or(0, |p| p + 1);
        order.insert(pos, *key);
    }
    order.into_iter().flat_map(|key| values.remove(&key).unwrap_or_default()).collect()
}

#[cfg(test)]
mod tests {
    use super::super::parse_notes_md;
    use super::super::tests::{SAMPLE, doc, task};
    use super::*;

    #[test]
    fn merge_tasks_three_way() {
        let base = doc(&[("a", false), ("b", false), ("c", false)], &[]);
        let ours = doc(&[("a", true), ("b", false), ("new ours", false)], &[]); // check a, delete c
        let theirs = doc(&[("a", false), ("theirs first", false), ("b", true), ("c", false)], &[]);
        let merged = merge_notes(&base, &ours, &theirs);
        assert_eq!(
            merged.tasks,
            [task("a", true), task("theirs first", false), task("b", true), task("new ours", false)]
        );
    }

    #[test]
    fn merge_tasks_delete_vs_change_and_double_add() {
        let base = doc(&[("a", false), ("b", false)], &[]);
        let ours = doc(&[("b", false), ("same", false)], &[]); // delete a
        let theirs = doc(&[("a", true), ("same", true)], &[]); // check a, delete b
        let merged = merge_notes(&base, &ours, &theirs);
        // a: deleted vs changed -> kept (changed). b: unchanged vs deleted -> gone. same: once.
        assert_eq!(merged.tasks, [task("a", true), task("same", true)]);
    }

    #[test]
    fn merge_notes_edits_and_conflicts() {
        let base = doc(&[], &["alpha one two", "beta three four", "gamma five", "delta"]);
        let ours =
            doc(&[], &["alpha one two plus more", "beta three four ours", "gamma five", "delta", "ours new"]);
        let theirs =
            doc(&[], &["alpha one two", "beta three four theirs", "theirs new", "delta changed a lot"]);
        let merged = merge_notes(&base, &ours, &theirs);
        assert_eq!(
            merged.notes,
            [
                "alpha one two plus more", // edited only by ours
                "beta three four ours",    // edited by both: both kept
                "beta three four theirs",
                // gamma: unchanged by ours, deleted by theirs -> gone
                "theirs new",          // theirs addition, after its predecessor
                "delta changed a lot", // edited only by theirs
                "ours new",
            ]
        );
    }

    #[test]
    fn merge_identity_and_symmetry_basics() {
        let base = parse_notes_md(SAMPLE);
        assert_eq!(merge_notes(&base, &base, &base), base);
        let mut edited = base.clone();
        edited.notes.push("x".into());
        edited.tasks[0].done = true;
        assert_eq!(merge_notes(&base, &edited, &base), edited);
        assert_eq!(merge_notes(&base, &base, &edited), edited);
        assert_eq!(merge_notes(&base, &edited, &edited), edited);
        // Deleted on both sides.
        let empty = NotesDoc::default();
        assert_eq!(merge_notes(&base, &empty, &empty), empty);
        // Edit vs delete keeps the edit.
        let mut ed = base.clone();
        ed.notes[1] = "Second note, revised".into();
        let mut del = base.clone();
        del.notes.remove(1);
        assert_eq!(merge_notes(&base, &del, &ed).notes, ed.notes);
    }

    #[test]
    fn reordering_in_the_app_reaches_the_file() {
        // Tasks and notes dragged into a new order in the app, file unchanged.
        let base = doc(&[("a", false), ("b", true), ("c", false)], &["one", "two"]);
        let ours = doc(&[("c", false), ("a", false), ("b", true)], &["two", "one"]);
        assert_eq!(merge_notes(&base, &ours, &base), ours);
        // ...while the file gained a task: it keeps its place after its predecessor.
        let theirs = doc(&[("a", false), ("b", true), ("c", false), ("d", false)], &["one", "two"]);
        let merged = merge_notes(&base, &ours, &theirs);
        assert_eq!(merged.tasks, [task("c", false), task("d", false), task("a", false), task("b", true)]);
        assert_eq!(merged.notes, ["two", "one"]);
    }

    #[test]
    fn unrelated_replacement_is_delete_plus_add() {
        let base = doc(&[], &["Call Bob"]);
        let ours = doc(&[], &["Buy milk"]);
        let theirs = doc(&[], &["Call Bob"]);
        assert_eq!(merge_notes(&base, &ours, &theirs).notes, ["Buy milk"]);
    }
}
