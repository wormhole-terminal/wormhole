//! What VoiceOver says for a group's head or a terminal row: one label that carries the state
//! word, the detail, the selection, the place in its set and the shortcut, so the glyphs and
//! keycaps themselves are never read.

use std::time::Duration;

use super::split::SplitMarks;
use super::state::RowState;
use super::wording::count;

/// What a terminal row is about, for its label.
#[derive(Clone, Copy, Debug)]
pub enum RowSpeech<'a> {
    /// A terminal with a coding agent: its state and what its second line says without the
    /// state word ("Approve cargo test?", "Editing sidebar.rs…").
    Agent { state: RowState, detail: Option<&'a str> },
    /// A terminal without an agent: the command in front (none: a bare shell), how long it has
    /// run, whether it rang its bell or printed while you weren't looking, and the exit code of
    /// the last command if it failed.
    Plain {
        command: Option<&'a str>,
        running: Option<Duration>,
        bell: bool,
        unread: bool,
        failed: Option<i32>,
    },
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
        RowSpeech::Plain { command, running, bell, unread, failed } => {
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
            if let Some(code) = failed {
                parts.push(format!("last command failed, exit {code}"));
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

/// A group's head: "Release 1.2, coloured blue, pinned, 3 terminals, expanded, 1 of 4". The colour is said
/// because the card's wash is all the eyes get of it; a folded group says how many of its
/// terminals need input or failed, since only the dot on its head tells the eyes. A group with a
/// folder says it after the count (`folder` is how the sidebar shows it, `~/Projects/app`), since
/// its tooltip is all the eyes get of it.
#[allow(clippy::too_many_arguments)]
pub fn band_label(
    label: &str,
    pinned: bool,
    color: Option<&str>,
    folder: Option<&str>,
    terminals: usize,
    alerts: usize,
    expanded: bool,
    position: usize,
    of: usize,
) -> String {
    let mut parts = vec![label.to_string()];
    parts.extend(color.map(|color| format!("coloured {}", color.to_lowercase())));
    if pinned {
        parts.push("pinned".to_string());
    }
    parts.push(if terminals == 0 {
        "no terminals".to_string()
    } else {
        count(terminals, "terminal", "terminals")
    });
    parts.extend(folder.map(|folder| format!("folder {folder}")));
    if !expanded && alerts > 0 {
        parts.push(format!("{alerts} need attention"));
    }
    parts.push(if expanded { "expanded" } else { "collapsed" }.to_string());
    parts.push(format!("{position} of {of}"));
    parts.join(", ")
}

/// The group's chevron: it folds the group or opens it again.
pub fn fold_label(name: &str, expanded: bool) -> String {
    format!("{} group {name}", if expanded { "Fold" } else { "Unfold" })
}

/// The group's label, which opens its popover (name, colour, actions).
pub fn edit_label(name: &str) -> String {
    format!("Edit group {name}")
}

/// The + on a group's head or its empty line.
pub fn new_in_group_label(name: &str) -> String {
    format!("New terminal in {name}")
}

/// The group switcher on the tab bar: "Group Release 1.2, 3 terminals, another group needs input".
/// The dot is all the eyes get of the last part, so it is said.
pub fn switcher_label(name: &str, terminals: usize, another_needs_you: bool) -> String {
    let mut parts = vec![format!("Group {name}"), terminals_in(terminals)];
    if another_needs_you {
        parts.push("another group needs input".to_string());
    }
    parts.join(", ")
}

/// A group's card in the switcher: "Review, 2 terminals, 2 need attention, current, 2 of 4".
pub fn switcher_card_label(
    name: &str,
    terminals: usize,
    alerts: usize,
    current: bool,
    position: usize,
    of: usize,
) -> String {
    let mut parts = vec![name.to_string(), terminals_in(terminals)];
    if alerts > 0 {
        parts.push(format!("{alerts} need attention"));
    }
    if current {
        parts.push("current".to_string());
    }
    parts.push(format!("{position} of {of}"));
    parts.join(", ")
}

/// A terminal's tab: "Fix pile fan-out, selected, 2 of 3". The tile's state is the row's words.
pub fn tab_label(name: &str, state: &str, selected: bool, position: usize, of: usize) -> String {
    let mut parts = vec![name.to_string(), state.to_string()];
    if selected {
        parts.push("selected".to_string());
    }
    parts.push(format!("{position} of {of}"));
    parts.join(", ")
}

fn terminals_in(n: usize) -> String {
    if n == 0 { "no terminals".to_string() } else { count(n, "terminal", "terminals") }
}

/// A row's state as a word: "needs input", "working", "finished", "failed".
pub fn state_word(state: RowState) -> &'static str {
    match state {
        RowState::NeedsYou => "needs input",
        RowState::Working => "working",
        RowState::Finished => "finished",
        RowState::Failed => "failed",
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
    use super::{
        Place, RowSpeech, SplitSpeech, band_label, edit_label, fold_label, new_in_group_label, row_label,
        row_label_in_split, spoken_clock, switcher_card_label, switcher_label, tab_label,
    };
    use crate::view::sidebar::{Bracket, RowState, SplitMarks};
    use std::time::Duration;

    fn place(selected: bool, position: usize, number: Option<usize>) -> Place {
        Place { selected, position, of: 3, number }
    }

    #[test]
    fn the_switcher_and_its_tabs_read_in_words() {
        assert_eq!(switcher_label("Release 1.2", 3, false), "Group Release 1.2, 3 terminals");
        assert_eq!(switcher_label("Docs", 0, true), "Group Docs, no terminals, another group needs input");
        assert_eq!(
            switcher_card_label("Review", 2, 2, true, 2, 4),
            "Review, 2 terminals, 2 need attention, current, 2 of 4"
        );
        assert_eq!(switcher_card_label("Docs", 0, 0, false, 4, 4), "Docs, no terminals, 4 of 4");
        assert_eq!(tab_label("cargo test", "working", true, 2, 3), "cargo test, working, selected, 2 of 3");
        assert_eq!(tab_label("zsh", "idle", false, 1, 3), "zsh, idle, 1 of 3");
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
            "diff perf, needs input, Approve cargo test?, 2 of 3, Command 2"
        );
        let idle = RowSpeech::Agent { state: RowState::Idle, detail: None };
        assert_eq!(row_label("zsh", idle, place(false, 3, None)), "zsh, idle, 3 of 3");
        let failed = RowSpeech::Agent { state: RowState::Failed, detail: Some("Rate limit reached") };
        assert_eq!(
            row_label("deploy", failed, place(false, 1, Some(1))),
            "deploy, failed, Rate limit reached, 1 of 3, Command 1"
        );
    }

    #[test]
    fn plain_terminals_say_what_runs() {
        let tests = RowSpeech::Plain {
            command: Some("cargo test -p wormhole-core"),
            running: Some(Duration::from_secs(72)),
            bell: false,
            unread: false,
            failed: None,
        };
        assert_eq!(
            row_label("tests", tests, place(false, 1, None)),
            "tests, running cargo, 1 minute 12, 1 of 3"
        );
        let prod = RowSpeech::Plain {
            command: Some("ssh deploy@prod-1"),
            running: None,
            bell: true,
            unread: false,
            failed: None,
        };
        assert_eq!(row_label("prod", prod, place(false, 2, None)), "prod, running ssh, bell rang, 2 of 3");
        let shell =
            RowSpeech::Plain { command: None, running: None, bell: false, unread: true, failed: None };
        assert_eq!(
            row_label("zsh", shell, place(false, 3, Some(4))),
            "zsh, shell, nothing running, unread output, 3 of 3, Command 4"
        );
        let failed =
            RowSpeech::Plain { command: None, running: None, bell: false, unread: true, failed: Some(101) };
        assert_eq!(
            row_label("build", failed, place(false, 1, None)),
            "build, shell, nothing running, unread output, last command failed, exit 101, 1 of 3"
        );
    }

    #[test]
    fn a_member_of_a_split_says_so_and_whether_it_is_on_screen() {
        let needs = RowSpeech::Agent { state: RowState::NeedsYou, detail: None };
        let split = |others, on_screen| Some(SplitSpeech { others, on_screen });
        assert_eq!(
            row_label_in_split("claude · tests", needs, place(false, 2, None), split(2, false)),
            "claude · tests, needs input, in a split with 2 others, 2 of 3"
        );
        assert_eq!(
            row_label_in_split("claude · tests", needs, place(false, 2, Some(2)), split(1, true)),
            "claude · tests, needs input, in a split with 1 other, on screen, 2 of 3, Command 2"
        );
        // The selected one is not "on screen": it is selected, and says so after the split.
        let plain =
            RowSpeech::Plain { command: None, running: None, bell: false, unread: false, failed: None };
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
    fn a_group_reads_its_colour_pin_count_and_place() {
        assert_eq!(
            band_label("Release 1.2", false, Some("Blue"), None, 3, 0, true, 1, 4),
            "Release 1.2, coloured blue, 3 terminals, expanded, 1 of 4"
        );
        assert_eq!(
            band_label("Review", true, Some("Pink"), None, 1, 0, false, 2, 4),
            "Review, coloured pink, pinned, 1 terminal, collapsed, 2 of 4"
        );
    }

    #[test]
    fn a_group_with_a_folder_says_it_after_the_count() {
        assert_eq!(
            band_label("Release 1.2", false, None, Some("~/Projects/app"), 3, 0, true, 1, 4),
            "Release 1.2, 3 terminals, folder ~/Projects/app, expanded, 1 of 4"
        );
    }

    #[test]
    fn a_pinned_group_says_so_with_no_terminal_in_it() {
        assert_eq!(
            band_label("Docs", true, None, None, 0, 0, true, 3, 4),
            "Docs, pinned, no terminals, expanded, 3 of 4"
        );
    }

    #[test]
    fn a_folded_group_says_what_its_dot_says() {
        assert_eq!(
            band_label("Experiments", false, None, None, 3, 1, false, 2, 3),
            "Experiments, 3 terminals, 1 need attention, collapsed, 2 of 3"
        );
        assert_eq!(
            band_label("Experiments", false, None, None, 3, 1, true, 2, 3),
            "Experiments, 3 terminals, expanded, 2 of 3",
            "open, its rows say it"
        );
    }

    #[test]
    fn the_groups_buttons_say_what_they_do() {
        assert_eq!(fold_label("Docs", true), "Fold group Docs");
        assert_eq!(fold_label("Docs", false), "Unfold group Docs");
        assert_eq!(edit_label("Docs"), "Edit group Docs");
        assert_eq!(new_in_group_label("Docs"), "New terminal in Docs");
    }

    #[test]
    fn running_times_are_spoken_in_words() {
        assert_eq!(spoken_clock(Duration::from_secs(1)), "1 second");
        assert_eq!(spoken_clock(Duration::from_secs(12)), "12 seconds");
        assert_eq!(spoken_clock(Duration::from_secs(72)), "1 minute 12");
        assert_eq!(spoken_clock(Duration::from_secs(3780)), "1 hour 3 minutes");
    }
}
