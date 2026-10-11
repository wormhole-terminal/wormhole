//! What VoiceOver says for a band or a terminal row: one label that carries the state
//! word, the detail, the selection, the place in its set and the shortcut, so the glyphs and
//! keycaps themselves are never read.

use std::time::Duration;

use super::filter::RowState;
use super::git::GroupGit;
use super::split::SplitMarks;
use super::text::count;

/// What a terminal row is about, for its label.
#[derive(Clone, Copy, Debug)]
pub enum RowSpeech<'a> {
    /// A terminal with a coding agent: its state and what its second line says without the
    /// state word ("Approve cargo test?", "Editing sidebar.rs…").
    Agent { state: RowState, detail: Option<&'a str> },
    /// A terminal without an agent: the command in front (none: a bare shell), how long it has
    /// run, whether it rang its bell or printed while you weren't looking.
    Plain { command: Option<&'a str>, running: Option<Duration>, bell: bool, unread: bool },
}

/// Where a row sits and what it is to the user.
#[derive(Clone, Copy, Debug)]
pub struct Place {
    pub selected: bool,
    /// 1-based, among the rows of its band (or of the flat list).
    pub position: usize,
    pub of: usize,
    /// Its ⌘ number.
    pub number: Option<usize>,
}

/// What a row says of the split its terminal is in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitSpeech {
    /// The other terminals in the split.
    pub others: usize,
    /// It is on screen beside the selected terminal.
    pub on_screen: bool,
}

impl SplitSpeech {
    /// From a row's marks: `None` for a terminal that isn't in a split.
    pub fn of(marks: SplitMarks) -> Option<SplitSpeech> {
        (marks.members > 1).then(|| SplitSpeech { others: marks.members - 1, on_screen: marks.on_screen })
    }
}

/// "sidebar polish, working, Editing sidebar.rs…, selected, 1 of 3, Command 1".
pub fn row_label(name: &str, speech: RowSpeech, place: Place) -> String {
    row_label_in_split(name, speech, place, None)
}

/// [`row_label`] for a terminal that may be in a split: after what it is doing, "in a split with 2
/// others", and "on screen" when it is shown beside the selected terminal ("claude · tests, needs
/// you, in a split with 2 others, on screen, 2 of 4").
pub fn row_label_in_split(name: &str, speech: RowSpeech, place: Place, split: Option<SplitSpeech>) -> String {
    let mut parts = vec![name.to_string()];
    match speech {
        RowSpeech::Agent { state, detail } => {
            parts.push(state_word(state).to_string());
            parts.extend(detail.map(str::trim).filter(|d| !d.is_empty()).map(str::to_string));
        }
        RowSpeech::Plain { command, running, bell, unread } => {
            match command.and_then(|command| command.split_whitespace().next()) {
                Some(program) => parts.push(format!("running {program}")),
                None => parts.push("shell, nothing running".to_string()),
            }
            parts.extend(running.map(spoken_clock));
            if bell {
                parts.push("bell rang".to_string());
            }
            if unread {
                parts.push("unread output".to_string());
            }
        }
    }
    if let Some(split) = split {
        parts.push(format!("in a split with {}", count(split.others, "other", "others")));
        if split.on_screen {
            parts.push("on screen".to_string());
        }
    }
    if place.selected {
        parts.push("selected".to_string());
    }
    parts.push(format!("{} of {}", place.position, place.of));
    parts.extend(place.number.map(|n| format!("Command {n}")));
    parts.join(", ")
}

/// "terminal, branch ui-unify, 3 changes, 3 terminals, expanded, 1 of 4"; a pinned folder says
/// so right after its name ("terminal, pinned, 0 terminals, expanded, 1 of 4"), and a coloured one
/// its colour ("terminal, coloured teal, 3 terminals, expanded, 1 of 4"): the band's tint is all
/// the eyes get of it, so VoiceOver has to be told.
#[allow(clippy::too_many_arguments)]
pub fn band_label(
    label: &str,
    pinned: bool,
    color: Option<&str>,
    git: Option<GroupGit>,
    terminals: usize,
    expanded: bool,
    position: usize,
    of: usize,
) -> String {
    let mut parts = vec![label.to_string()];
    if pinned {
        parts.push("pinned".to_string());
    }
    parts.extend(color.map(|color| format!("coloured {}", color.to_lowercase())));
    if let Some(git) = git {
        parts.push(match git.branch {
            Some(branch) => format!("branch {branch}"),
            None => "detached HEAD".to_string(),
        });
        parts.extend(git.changes.filter(|&n| n > 0).map(|n| count(n, "change", "changes")));
    }
    parts.push(count(terminals, "terminal", "terminals"));
    parts.push(if expanded { "expanded" } else { "collapsed" }.to_string());
    parts.push(format!("{position} of {of}"));
    parts.join(", ")
}

/// A row's state as a word: "needs you", "working", "finished".
pub fn state_word(state: RowState) -> &'static str {
    match state {
        RowState::NeedsYou => "needs you",
        RowState::Working => "working",
        RowState::Finished => "finished",
        RowState::Idle => "idle",
        RowState::Exited => "exited",
    }
}

/// A running time as it is spoken: "12 seconds", "1 minute 12", "1 hour 3 minutes".
pub fn spoken_clock(elapsed: Duration) -> String {
    let secs = elapsed.as_secs();
    let (hours, minutes, seconds) = (secs / 3600, secs / 60 % 60, secs % 60);
    match (hours, minutes) {
        (0, 0) => count(seconds as usize, "second", "seconds"),
        (0, _) => format!("{} {seconds}", count(minutes as usize, "minute", "minutes")),
        _ => format!(
            "{} {}",
            count(hours as usize, "hour", "hours"),
            count(minutes as usize, "minute", "minutes")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{Place, RowSpeech, SplitSpeech, band_label, row_label, row_label_in_split, spoken_clock};
    use crate::sidebar::{Bracket, GroupGit, RowState, SplitMarks};
    use std::time::Duration;

    fn place(selected: bool, position: usize, number: Option<usize>) -> Place {
        Place { selected, position, of: 3, number }
    }

    #[test]
    fn rows_read_as_voiceover_says() {
        let working = RowSpeech::Agent { state: RowState::Working, detail: Some("Editing sidebar.rs…") };
        assert_eq!(
            row_label("sidebar polish", working, place(true, 1, Some(1))),
            "sidebar polish, working, Editing sidebar.rs…, selected, 1 of 3, Command 1"
        );
        let needs = RowSpeech::Agent { state: RowState::NeedsYou, detail: Some("Approve cargo test?") };
        assert_eq!(
            row_label("diff perf", needs, place(false, 2, Some(2))),
            "diff perf, needs you, Approve cargo test?, 2 of 3, Command 2"
        );
        let idle = RowSpeech::Agent { state: RowState::Idle, detail: None };
        assert_eq!(row_label("zsh", idle, place(false, 3, None)), "zsh, idle, 3 of 3");
    }

    #[test]
    fn plain_terminals_say_what_runs() {
        let tests = RowSpeech::Plain {
            command: Some("cargo test -p wormhole-core"),
            running: Some(Duration::from_secs(72)),
            bell: false,
            unread: false,
        };
        assert_eq!(
            row_label("tests", tests, place(false, 1, None)),
            "tests, running cargo, 1 minute 12, 1 of 3"
        );
        let prod =
            RowSpeech::Plain { command: Some("ssh deploy@prod-1"), running: None, bell: true, unread: false };
        assert_eq!(row_label("prod", prod, place(false, 2, None)), "prod, running ssh, bell rang, 2 of 3");
        let shell = RowSpeech::Plain { command: None, running: None, bell: false, unread: true };
        assert_eq!(
            row_label("zsh", shell, place(false, 3, Some(4))),
            "zsh, shell, nothing running, unread output, 3 of 3, Command 4"
        );
    }

    #[test]
    fn a_member_of_a_split_says_so_and_whether_it_is_on_screen() {
        let needs = RowSpeech::Agent { state: RowState::NeedsYou, detail: None };
        let split = |others, on_screen| Some(SplitSpeech { others, on_screen });
        assert_eq!(
            row_label_in_split("claude · tests", needs, place(false, 2, None), split(2, false)),
            "claude · tests, needs you, in a split with 2 others, 2 of 3"
        );
        assert_eq!(
            row_label_in_split("claude · tests", needs, place(false, 2, Some(2)), split(1, true)),
            "claude · tests, needs you, in a split with 1 other, on screen, 2 of 3, Command 2"
        );
        // The selected one is not "on screen": it is selected, and says so after the split.
        let plain = RowSpeech::Plain { command: None, running: None, bell: false, unread: false };
        assert_eq!(
            row_label_in_split("zsh", plain, place(true, 1, Some(1)), split(2, false)),
            "zsh, shell, nothing running, in a split with 2 others, selected, 1 of 3, Command 1"
        );
        // Outside a split nothing is said, and the plain label is the same one.
        assert_eq!(
            row_label_in_split("claude", needs, place(false, 1, None), None),
            row_label("claude", needs, place(false, 1, None))
        );
    }

    #[test]
    fn a_rows_marks_become_its_split_words() {
        let marks =
            |members, on_screen| SplitMarks { bracket: Some(Bracket::Mid), chain: false, on_screen, members };
        assert_eq!(SplitSpeech::of(marks(3, false)), Some(SplitSpeech { others: 2, on_screen: false }));
        assert_eq!(SplitSpeech::of(marks(2, true)), Some(SplitSpeech { others: 1, on_screen: true }));
        assert_eq!(SplitSpeech::of(SplitMarks::default()), None);
        assert_eq!(SplitSpeech::of(marks(1, false)), None, "a split of one is none");
    }

    #[test]
    fn bands_read_their_branch_changes_and_count() {
        let git = GroupGit { branch: Some("ui-unify"), changes: Some(3) };
        assert_eq!(
            band_label("terminal", false, None, Some(git), 3, true, 1, 4),
            "terminal, branch ui-unify, 3 changes, 3 terminals, expanded, 1 of 4"
        );
        assert_eq!(
            band_label("dotfiles", false, None, None, 1, false, 3, 4),
            "dotfiles, 1 terminal, collapsed, 3 of 4"
        );
        let detached = GroupGit { branch: None, changes: Some(0) };
        assert_eq!(
            band_label("bisect", false, None, Some(detached), 2, true, 1, 1),
            "bisect, detached HEAD, 2 terminals, expanded, 1 of 1"
        );
    }

    #[test]
    fn a_coloured_folder_says_its_colour_after_its_name() {
        assert_eq!(
            band_label("app", false, Some("Teal"), None, 3, true, 2, 4),
            "app, coloured teal, 3 terminals, expanded, 2 of 4"
        );
        let git = GroupGit { branch: Some("main"), changes: Some(0) };
        assert_eq!(
            band_label("app", true, Some("Pink"), Some(git), 0, false, 1, 4),
            "app, pinned, coloured pink, branch main, 0 terminals, collapsed, 1 of 4"
        );
    }

    #[test]
    fn a_pinned_folder_says_so_even_with_no_terminal_in_it() {
        assert_eq!(
            band_label("notes", true, None, None, 0, true, 1, 2),
            "notes, pinned, 0 terminals, expanded, 1 of 2"
        );
        let git = GroupGit { branch: Some("main"), changes: Some(0) };
        assert_eq!(
            band_label("terminal", true, None, Some(git), 2, false, 1, 2),
            "terminal, pinned, branch main, 2 terminals, collapsed, 1 of 2"
        );
    }

    #[test]
    fn running_times_are_spoken_in_words() {
        assert_eq!(spoken_clock(Duration::from_secs(1)), "1 second");
        assert_eq!(spoken_clock(Duration::from_secs(12)), "12 seconds");
        assert_eq!(spoken_clock(Duration::from_secs(72)), "1 minute 12");
        assert_eq!(spoken_clock(Duration::from_secs(3780)), "1 hour 3 minutes");
    }
}
