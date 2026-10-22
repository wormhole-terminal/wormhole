//! The right panel's list rules: where done next steps fold, when a Changes
//! section starts folded and what it says then, how much of an open note shows, and how old a
//! commit reads. The app draws what these decide.

use crate::limits::{DONE_FOLD_ABOVE, FOLD_ABOVE, NOTE_OPEN_MAX_LINES, count_grouped};

/// One line of the Next steps list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepRow {
    /// The step at this index of the list.
    Step(usize),
    /// A new step being written, where it will go in.
    Draft,
    /// "N done": the done steps, folded under the open ones.
    Done(usize),
}

/// The Next steps list as drawn. With more than [`DONE_FOLD_ABOVE`] done steps those leave their
/// places and fold into one "N done" row after the open ones, with the done steps under it while
/// `show_done`; with fewer, every step stays where it is. `draft` is the index a new step goes in
/// at: it is drawn before the step now there or, while the done steps are folded away, before the
/// first open step at or after it.
pub fn steps(done: &[bool], draft: Option<usize>, show_done: bool) -> Vec<StepRow> {
    let done_count = done.iter().filter(|&&d| d).count();
    let mut rows = Vec::with_capacity(done.len() + 2);
    if done_count <= DONE_FOLD_ABOVE {
        for ix in 0..done.len() {
            if draft == Some(ix) {
                rows.push(StepRow::Draft);
            }
            rows.push(StepRow::Step(ix));
        }
        if draft.is_some_and(|at| at >= done.len()) {
            rows.push(StepRow::Draft);
        }
        return rows;
    }
    let mut draft = draft;
    for ix in (0..done.len()).filter(|&ix| !done[ix]) {
        if draft.is_some_and(|at| at <= ix) {
            rows.push(StepRow::Draft);
            draft = None;
        }
        rows.push(StepRow::Step(ix));
    }
    if draft.is_some() {
        rows.push(StepRow::Draft);
    }
    rows.push(StepRow::Done(done_count));
    if show_done {
        rows.extend((0..done.len()).filter(|&ix| done[ix]).map(StepRow::Step));
    }
    rows
}

/// Whether a Changes section of `files` files is folded: as the user last left it (`choice`),
/// or, until they fold or unfold it, when it holds more than [`FOLD_ABOVE`].
pub fn section_folded(files: usize, choice: Option<bool>) -> bool {
    choice.unwrap_or(files > FOLD_ABOVE)
}

/// Whether the Changes list keeps untracked files in a band of their own. The "group untracked"
/// setting says so for the list against HEAD; compared with a branch the list is
/// everything since the branch split off, as a pull request shows it, and untracked files are
/// part of that one list whatever the setting says.
pub fn untracked_apart(setting: bool, compared: bool) -> bool {
    setting && !compared
}

/// What a section folded for its size says under its band: where the files are when one folder
/// at the top of the repository holds most of them (for untracked files that usually means a
/// build folder nobody ignored), then why the section is folded.
pub fn folded_note<'a>(paths: impl IntoIterator<Item = &'a str>, untracked: bool) -> String {
    let (mut total, mut by_folder) = (0usize, std::collections::HashMap::<&str, usize>::new());
    for path in paths {
        total += 1;
        if let Some((top, _)) = path.split_once('/') {
            *by_folder.entry(top).or_default() += 1;
        }
    }
    let why = format!("Folded because there are more than {}.", count_grouped(FOLD_ABOVE));
    // The biggest folder, the first by name on a tie, so the note doesn't change between refreshes.
    let crowded = by_folder.into_iter().max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0)));
    match crowded.filter(|&(_, n)| n * 2 > total) {
        Some((folder, n)) => {
            let ignored = if untracked { ", which isn't ignored" } else { "" };
            format!("{} of them are in {folder}/{ignored}. {why}", count_grouped(n))
        }
        None => why,
    }
}

/// How many lines of an open note show: all while it is being edited or after "Show all"
/// (`None`), otherwise at most [`NOTE_OPEN_MAX_LINES`] (`None` too when that's all it has).
pub fn note_cap(lines: usize, editing: bool, show_all: bool) -> Option<usize> {
    (!editing && !show_all && lines > NOTE_OPEN_MAX_LINES).then_some(NOTE_OPEN_MAX_LINES)
}

/// How long ago something happened, as the commit list says it on the right edge: "now",
/// "12m", "3h", "2d", "5w", "4mo", "2y". A time in the future (a skewed clock) is "now".
pub fn age(seconds: i64) -> String {
    const MINUTE: i64 = 60;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    const WEEK: i64 = 7 * DAY;
    // A Gregorian year and a twelfth of it.
    const YEAR: i64 = 31_556_952;
    const MONTH: i64 = YEAR / 12;
    match seconds.max(0) {
        s if s < MINUTE => "now".into(),
        s if s < HOUR => format!("{}m", s / MINUTE),
        s if s < DAY => format!("{}h", s / HOUR),
        s if s < WEEK => format!("{}d", s / DAY),
        s if s < MONTH => format!("{}w", s / WEEK),
        s if s < YEAR => format!("{}mo", s / MONTH),
        s => format!("{}y", s / YEAR),
    }
}

#[cfg(test)]
mod tests {
    use super::{StepRow, age, folded_note, note_cap, section_folded, steps, untracked_apart};
    use crate::limits::{DONE_FOLD_ABOVE, FOLD_ABOVE, NOTE_OPEN_MAX_LINES};
    use StepRow::{Done, Draft, Step};

    #[test]
    fn a_few_done_steps_stay_where_they_are() {
        let done = [true, false, true, false];
        assert_eq!(steps(&done, None, false), [Step(0), Step(1), Step(2), Step(3)]);
        // A new step is drawn where it goes in, or last.
        assert_eq!(steps(&done, Some(2), false), [Step(0), Step(1), Draft, Step(2), Step(3)]);
        assert_eq!(steps(&done, Some(4), false), [Step(0), Step(1), Step(2), Step(3), Draft]);
        assert_eq!(steps(&[], Some(0), false), [Draft]);
    }

    /// "50 next steps": 38 open, 12 done fold into one row at the end.
    #[test]
    fn more_done_steps_than_the_limit_fold_after_the_open_ones() {
        let mut done = vec![false; 50];
        for ix in (0..50).step_by(4).take(12) {
            done[ix] = true;
        }
        let rows = steps(&done, None, false);
        assert_eq!(rows.len(), 38 + 1);
        assert_eq!(rows.last(), Some(&Done(12)));
        assert!(rows[..38].iter().all(|row| matches!(row, Step(ix) if !done[*ix])));
        // Unfolded, the done ones follow the fold row in their own order.
        let open = steps(&done, None, true);
        assert_eq!(open.len(), 50 + 1);
        assert_eq!(open[39..], (0..50).step_by(4).take(12).map(Step).collect::<Vec<_>>()[..]);
    }

    #[test]
    fn the_limit_itself_does_not_fold() {
        let done: Vec<bool> = (0..DONE_FOLD_ABOVE + 1).map(|ix| ix < DONE_FOLD_ABOVE).collect();
        assert!(!steps(&done, None, false).iter().any(|row| matches!(row, Done(_))));
        let done = vec![true; DONE_FOLD_ABOVE + 1];
        assert_eq!(steps(&done, None, false), [Done(DONE_FOLD_ABOVE + 1)]);
    }

    #[test]
    fn a_draft_goes_among_the_open_steps_while_the_done_ones_are_folded() {
        // Open: 1, 3. Done: 0, 2, 4, 5, 6, 7.
        let done = [true, false, true, false, true, true, true, true];
        // The + adds at the end: after the last open step, before the fold.
        assert_eq!(steps(&done, Some(8), false), [Step(1), Step(3), Draft, Done(6)]);
        // ↵ in step 1 writes the next one at 2, which is a done step's place: before step 3.
        assert_eq!(steps(&done, Some(2), false), [Step(1), Draft, Step(3), Done(6)]);
        assert_eq!(steps(&done, Some(0), false), [Draft, Step(1), Step(3), Done(6)]);
    }

    #[test]
    fn compared_with_a_branch_the_list_is_one_group_whatever_the_setting() {
        assert!(untracked_apart(true, false), "against HEAD the setting decides");
        assert!(!untracked_apart(false, false));
        assert!(!untracked_apart(true, true), "against a branch untracked files join the list");
        assert!(!untracked_apart(false, true));
    }

    #[test]
    fn a_big_section_starts_folded_until_the_user_says_otherwise() {
        assert!(!section_folded(FOLD_ABOVE, None));
        assert!(section_folded(FOLD_ABOVE + 1, None));
        assert!(!section_folded(1_284, Some(false)));
        assert!(section_folded(3, Some(true)));
    }

    #[test]
    fn a_folded_section_says_where_most_of_its_files_are() {
        let mut paths: Vec<String> = (0..1_090).map(|n| format!("target/debug/deps/{n}.o")).collect();
        paths.extend((0..12).map(|n| format!("src/{n}.rs")));
        assert_eq!(
            folded_note(paths.iter().map(String::as_str), true),
            "1,090 of them are in target/, which isn't ignored. Folded because there are more than 200."
        );
        assert_eq!(
            folded_note(paths.iter().map(String::as_str), false),
            "1,090 of them are in target/. Folded because there are more than 200."
        );
    }

    #[test]
    fn without_a_crowded_folder_the_note_only_says_why() {
        // Half is not most; files at the top level are in no folder.
        let paths = ["a/1", "a/2", "b/1", "b/2", "top.rs", "other.rs"];
        assert_eq!(folded_note(paths, true), "Folded because there are more than 200.");
        assert_eq!(folded_note([], false), "Folded because there are more than 200.");
    }

    #[test]
    fn an_open_note_shows_twelve_lines_until_it_is_edited_or_shown_all() {
        assert_eq!(note_cap(NOTE_OPEN_MAX_LINES, false, false), None);
        assert_eq!(note_cap(31, false, false), Some(NOTE_OPEN_MAX_LINES));
        assert_eq!(note_cap(31, true, false), None, "editing lifts the cap");
        assert_eq!(note_cap(31, false, true), None, "so does Show all");
    }

    #[test]
    fn ages_read_like_the_commit_list() {
        assert_eq!(age(-5), "now");
        assert_eq!(age(59), "now");
        assert_eq!(age(31 * 60), "31m");
        assert_eq!(age(3 * 3600 + 59), "3h");
        assert_eq!(age(86_400), "1d");
        assert_eq!(age(15 * 86_400), "2w");
        assert_eq!(age(95 * 86_400), "3mo");
        assert_eq!(age(800 * 86_400), "2y");
    }
}
