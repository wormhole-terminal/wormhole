//! Which lines of the terminal list slide when the list changes, and how far. Pure geometry: the
//! app measures where the lines were, this works out where they land and what each has to be
//! shifted by to start from its old place.

use std::collections::HashMap;
use std::hash::Hash;

/// A line of the list as last drawn: what it is, where its top was (from the content's start)
/// and how tall it was.
#[derive(Clone, Debug, PartialEq)]
pub struct Drawn<K> {
    pub key: K,
    pub top: f32,
    pub height: f32,
}

/// A line that changes place: where its top was (as drawn, from the content's start) and where it is
/// now. The two are places in the content, not a distance, so a move that is retargeted while it is
/// under way can carry on from where the line is.
#[derive(Clone, Debug, PartialEq)]
pub struct Move<K> {
    pub key: K,
    /// Where it started. A move longer than the plan's `limit` starts that far from `to`.
    pub from: f32,
    pub to: f32,
}

impl<K> Move<K> {
    /// How far below (positive) or above (negative) its new place it starts: the offset that goes to 0.
    #[must_use]
    pub fn shift(&self) -> f32 {
        self.from - self.to
    }
}

/// What a change of the list does to its lines.
#[derive(Clone, Debug, PartialEq)]
pub struct Moves<K> {
    /// Each line that changes place.
    pub moved: Vec<Move<K>>,
    /// Lines that were not in the list before.
    pub entering: Vec<K>,
}

impl<K> Default for Moves<K> {
    fn default() -> Self {
        Moves { moved: Vec::new(), entering: Vec::new() }
    }
}

/// A shift smaller than this is a rounding of the layout, not a move.
const MIN_MOVE: f32 = 0.5;

/// How the list `now` (each line's key and, for one that was not drawn before, its likely
/// height) differs from `before`, laid out from `pad` down. A move is capped at `limit`, so a
/// terminal that jumps across a long list still slides, in a short way.
pub fn plan_moves<K: Eq + Hash + Clone>(
    before: &[Drawn<K>],
    now: &[(K, f32)],
    pad: f32,
    limit: f32,
) -> Moves<K> {
    let drawn: HashMap<&K, &Drawn<K>> = before.iter().map(|line| (&line.key, line)).collect();
    let mut plan = Moves::default();
    let mut top = pad;
    for (key, likely) in now {
        match drawn.get(key) {
            Some(line) => {
                let shift = (line.top - top).clamp(-limit, limit);
                if shift.abs() >= MIN_MOVE {
                    plan.moved.push(Move { key: key.clone(), from: top + shift, to: top });
                }
                top += line.height;
            }
            None => {
                plan.entering.push(key.clone());
                top += likely;
            }
        }
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::{Drawn, Move, Moves, plan_moves};

    /// Lines of 10 from `pad` 4 down.
    fn drawn(keys: &[&'static str]) -> Vec<Drawn<&'static str>> {
        keys.iter()
            .enumerate()
            .map(|(at, &key)| Drawn { key, top: 4.0 + 10.0 * at as f32, height: 10.0 })
            .collect()
    }

    fn now(keys: &[&'static str]) -> Vec<(&'static str, f32)> {
        keys.iter().map(|&key| (key, 10.0)).collect()
    }

    #[test]
    fn nothing_changed_nothing_moves() {
        let plan = plan_moves(&drawn(&["a", "b", "c"]), &now(&["a", "b", "c"]), 4.0, 400.0);
        assert_eq!(plan, Moves::default());
    }

    #[test]
    fn a_line_added_at_the_end_moves_no_other() {
        let plan = plan_moves(&drawn(&["a", "b"]), &now(&["a", "b", "c"]), 4.0, 400.0);
        assert!(plan.moved.is_empty());
        assert_eq!(plan.entering, ["c"]);
    }

    #[test]
    fn pinning_a_line_to_the_top_moves_it_up_and_the_ones_it_passes_down() {
        // c goes from the bottom to the top.
        let plan = plan_moves(&drawn(&["a", "b", "c"]), &now(&["c", "a", "b"]), 4.0, 400.0);
        let moved: std::collections::HashMap<_, _> =
            plan.moved.iter().map(|line| (line.key, line.shift())).collect();
        assert_eq!(moved["c"], 20.0, "it starts two lines below its place");
        assert_eq!(moved["a"], -10.0, "the others start one line above theirs");
        assert_eq!(moved["b"], -10.0);
        assert!(plan.entering.is_empty());
    }

    #[test]
    fn a_line_inserted_above_pushes_the_rest_and_one_removed_pulls_them() {
        let plan = plan_moves(&drawn(&["a", "b"]), &now(&["x", "a", "b"]), 4.0, 400.0);
        assert_eq!(
            plan.moved,
            [Move { key: "a", from: 4.0, to: 14.0 }, Move { key: "b", from: 14.0, to: 24.0 }]
        );
        assert_eq!(plan.entering, ["x"]);
        let plan = plan_moves(&drawn(&["a", "b", "c"]), &now(&["b", "c"]), 4.0, 400.0);
        assert_eq!(
            plan.moved,
            [Move { key: "b", from: 14.0, to: 4.0 }, Move { key: "c", from: 24.0, to: 14.0 }]
        );
    }

    #[test]
    fn a_long_way_is_capped_and_a_new_line_takes_its_likely_height() {
        let before: Vec<_> =
            (0..60).map(|at| Drawn { key: at, top: 4.0 + 10.0 * at as f32, height: 10.0 }).collect();
        let mut after: Vec<(i32, f32)> = (0..60).map(|at| (at, 10.0)).collect();
        let last = after.pop().map(|line| line.0).unwrap_or(0);
        after.insert(0, (last, 10.0));
        let plan = plan_moves(&before, &after, 4.0, 100.0);
        assert_eq!(plan.moved[0], Move { key: 59, from: 104.0, to: 4.0 }, "capped");
        let plan = plan_moves(&drawn(&["a", "b"]), &[("new", 25.0), ("a", 10.0), ("b", 10.0)], 4.0, 400.0);
        let shifts: Vec<f32> = plan.moved.iter().map(Move::shift).collect();
        assert_eq!(shifts, [-25.0, -25.0]);
    }
}
