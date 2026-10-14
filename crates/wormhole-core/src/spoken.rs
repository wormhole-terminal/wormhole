//! What VoiceOver says for the right panel's tabs and section bands and for the status bar.
//! A label carries in words what the element's glyph, count and keycap show; the role
//! and state the app gives the element add the rest ("selected, tab, 2 of 4", "expanded,
//! button"), so those are never part of a label. The status bar's items are spoken whole at any
//! width: a narrow window drops their words on screen ("4m · 1"), never in speech.
//!
//! The sidebar's labels are `sidebar::spoken`; what a terminal's change announces is
//! `notify::announce`.

use std::time::{Duration, Instant};

use crate::layout::status_bar::{Entry, Summary};
use crate::sidebar::text::count;
use crate::terminal::BlockState;

/// A command block as VoiceOver announces it when ⌘↑ / ⌘↓ lands on it, and as the readout
/// beside a hovered block is labelled: the command, how it went and how long it took. "cargo
/// test, failed with exit code 101, 3.5 seconds"; "cargo fmt, finished, 1.2 seconds"; "cargo
/// build, running"; "prompt" for one where nothing ran.
pub fn command_block(command: &str, state: BlockState) -> String {
    let command = command.lines().next().unwrap_or_default().trim();
    let mut parts: Vec<String> = Vec::new();
    if !command.is_empty() {
        parts.push(command.to_string());
    }
    match state {
        BlockState::Prompt => parts.push("prompt".to_string()),
        BlockState::Running { .. } => parts.push("running".to_string()),
        BlockState::Finished { code, took } => {
            match code {
                Some(0) => parts.push("finished".to_string()),
                Some(code) => parts.push(format!("failed with exit code {code}")),
                None if took.is_some() => parts.push("finished".to_string()),
                None if command.is_empty() => parts.push("empty prompt".to_string()),
                None => {}
            }
            if let Some(took) = took {
                parts.push(spoken_took(took));
            }
        }
    }
    parts.join(", ")
}

/// How long a command took, in words: "0.4 seconds", "3.5 seconds", "12 seconds", "1 minute 5
/// seconds", "2 hours 3 minutes".
pub fn spoken_took(took: Duration) -> String {
    let secs = took.as_secs();
    match secs {
        0..10 => format!("{:.1} seconds", took.as_secs_f64()),
        10..60 => count(secs as usize, "second", "seconds"),
        60..3600 => {
            let minutes = count((secs / 60) as usize, "minute", "minutes");
            match secs % 60 {
                0 => minutes,
                rest => format!("{minutes} {}", count(rest as usize, "second", "seconds")),
            }
        }
        _ => spoken_elapsed(took),
    }
}

/// A right-panel tab as VoiceOver names it (the role adds "tab", the state "selected", the place
/// "2 of 4").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelTab {
    /// The next steps still to do and the notes with something in them.
    Notes {
        open_steps: usize,
        notes: usize,
    },
    /// The files changed in the terminal's repository; `None` while that isn't known, or outside
    /// a repository.
    Changes {
        files: Option<usize>,
    },
    History,
    Project,
    /// The coding agents' plan usage limits.
    Usage,
}

/// "Changes, 3 changed files", "Notes, 2 open next steps, 1 note". An empty Notes tab is the
/// usual case and says only its name; a repository without changes says so, since a clean tree
/// is news.
pub fn panel_tab(tab: PanelTab) -> String {
    match tab {
        PanelTab::Notes { open_steps, notes } => {
            let mut parts = vec!["Notes".to_string()];
            if open_steps > 0 {
                parts.push(count(open_steps, "open next step", "open next steps"));
            }
            if notes > 0 {
                parts.push(count(notes, "note", "notes"));
            }
            parts.join(", ")
        }
        PanelTab::Changes { files: None } => "Changes".to_string(),
        PanelTab::Changes { files: Some(0) } => "Changes, no changes".to_string(),
        PanelTab::Changes { files: Some(n) } => {
            format!("Changes, {}", count(n, "changed file", "changed files"))
        }
        PanelTab::History => "History".to_string(),
        PanelTab::Project => "Project".to_string(),
        PanelTab::Usage => "Usage".to_string(),
    }
}

/// What a section band counts, for the words after its number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Counted {
    /// Next steps not yet done: "2 open".
    Open,
    /// "2 notes".
    Notes,
    /// Changed or untracked files: "3 files".
    Files,
    /// "12 commits".
    Commits,
    /// Anything else, such as a project's actions or prompts: "4 items".
    Items,
}

/// A section band of the right panel ("Next steps", "Notes", "Changes", "Untracked", a project's
/// sections): its title and count in words, "Next steps, 2 open". The band hides a zero count;
/// speech says it ("Notes, no notes"), so an empty section isn't mistaken for one whose count
/// wasn't read. The band's role adds "button" and its state "expanded".
pub fn section_band(title: &str, n: usize, counted: Counted) -> String {
    let words = match counted {
        Counted::Open if n == 0 => "none open".to_string(),
        Counted::Open => format!("{n} open"),
        Counted::Notes => some_or_none(n, "note", "notes"),
        Counted::Files => some_or_none(n, "file", "files"),
        Counted::Commits => some_or_none(n, "commit", "commits"),
        Counted::Items => some_or_none(n, "item", "items"),
    };
    format!("{title}, {words}")
}

/// A pane's header in a split tab: what runs in it, its folder when the header shows one,
/// its place among the tab's panes in reading order, and whether it has the keyboard ("claude,
/// terminal, pane 1 of 3, focused"). The buttons on the header are named by their tooltips.
pub fn pane(name: &str, folder: Option<&str>, position: usize, of: usize, focused: bool) -> String {
    let mut parts = vec![name.to_string()];
    parts.extend(folder.filter(|f| !f.is_empty()).map(str::to_string));
    parts.push(format!("pane {position} of {of}"));
    if focused {
        parts.push("focused".to_string());
    }
    parts.join(", ")
}

/// The branch item: "Branch ui-unify, 3 changes". The bar names a detached HEAD "HEAD" (no
/// branch may have that name), which is said the way the sidebar's band says it.
pub fn status_branch(branch: &str, changes: usize) -> String {
    let name = if branch == "HEAD" { "Detached HEAD".to_string() } else { format!("Branch {branch}") };
    format!("{name}, {}", some_or_none(changes, "change", "changes"))
}

/// The context meter: "Agent context, 62 percent".
pub fn status_context(percent: u32) -> String {
    format!("Agent context, {percent} percent")
}

/// The size item: "zsh, 93 columns by 40 rows".
pub fn status_size(shell: &str, cols: usize, rows: usize) -> String {
    format!("{shell}, {} by {}", count(cols, "column", "columns"), count(rows, "row", "rows"))
}

/// The working item of `summary` (made by `Summary::of(entries)`): one terminal says who works
/// and for how long ("Claude working, 4 minutes", "1 terminal working, less than a minute"
/// without an agent), several how many ("2 terminals working").
pub fn status_working(summary: &Summary, entries: &[Entry], now: Instant) -> Option<String> {
    match summary.working.as_slice() {
        [] => None,
        [one] => {
            let entry = &entries[*one];
            let time = spoken_elapsed(now.saturating_duration_since(entry.since));
            Some(match entry.agent {
                Some(agent) => format!("{agent} working, {time}"),
                None => format!("1 terminal working, {time}"),
            })
        }
        many => Some(format!("{} working", count(many.len(), "terminal", "terminals"))),
    }
}

/// The needs-you item: "1 terminal needs you", "2 terminals need you".
pub fn status_needs_you(summary: &Summary) -> Option<String> {
    let n = summary.needs_you.len();
    (n > 0).then(|| terminals_need_you(n))
}

/// The done item: "1 terminal finished", "3 terminals finished".
pub fn status_done(summary: &Summary) -> Option<String> {
    let n = summary.done.len();
    (n > 0).then(|| terminals_finished(n))
}

/// The failed item: "1 terminal failed", "3 terminals failed".
pub fn status_failed(summary: &Summary) -> Option<String> {
    let n = summary.failed.len();
    (n > 0).then(|| terminals_failed(n))
}

/// A spoken duration at the bar's minute granularity ("4m" on screen, "<1m" in the first
/// minute): "less than a minute", "4 minutes", "1 hour 5 minutes", and "2 hours" rather than "2
/// hours 0 minutes".
pub fn spoken_elapsed(time: Duration) -> String {
    let secs = time.as_secs();
    match secs {
        0..60 => "less than a minute".to_string(),
        60..3600 => count((secs / 60) as usize, "minute", "minutes"),
        _ => {
            let hours = count((secs / 3600) as usize, "hour", "hours");
            match secs % 3600 / 60 {
                0 => hours,
                minutes => format!("{hours} {}", count(minutes as usize, "minute", "minutes")),
            }
        }
    }
}

/// "2 terminals need you"; the status bar and an announced burst say it alike.
pub fn terminals_need_you(n: usize) -> String {
    if n == 1 { "1 terminal needs you".to_string() } else { format!("{n} terminals need you") }
}

/// "3 terminals finished".
pub(crate) fn terminals_finished(n: usize) -> String {
    format!("{} finished", count(n, "terminal", "terminals"))
}

/// "3 terminals failed".
pub(crate) fn terminals_failed(n: usize) -> String {
    format!("{} failed", count(n, "terminal", "terminals"))
}

/// "1 file", "3 files", and "no files" for none.
fn some_or_none(n: usize, one: &str, many: &str) -> String {
    if n == 0 { format!("no {many}") } else { count(n, one, many) }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        Counted, PanelTab, command_block, panel_tab, section_band, spoken_elapsed, spoken_took,
        status_branch, status_context, status_done, status_failed, status_needs_you, status_size,
        status_working,
    };
    use crate::layout::status_bar::{Activity, Detail, Entry, Summary};
    use crate::terminal::BlockState;

    #[test]
    fn a_command_block_is_said_with_its_result_and_its_time() {
        let secs = Duration::from_secs;
        let finished = |code, took| BlockState::Finished { code, took };
        assert_eq!(
            command_block("cargo test", finished(Some(101), Some(Duration::from_millis(3500)))),
            "cargo test, failed with exit code 101, 3.5 seconds"
        );
        assert_eq!(
            command_block("cargo fmt", finished(Some(0), Some(secs(72)))),
            "cargo fmt, finished, 1 minute 12 seconds"
        );
        assert_eq!(command_block("make", finished(Some(0), None)), "make, finished", "a shell without `C`");
        assert_eq!(
            command_block("cargo build", BlockState::Running { since: Instant::now() }),
            "cargo build, running"
        );
        assert_eq!(command_block("", BlockState::Prompt), "prompt");
        assert_eq!(command_block("", finished(None, None)), "empty prompt");
        assert_eq!(
            command_block("for x in a\necho $x", BlockState::Prompt),
            "for x in a, prompt",
            "the first line"
        );
        assert_eq!(spoken_took(Duration::from_millis(420)), "0.4 seconds");
        assert_eq!(spoken_took(secs(12)), "12 seconds");
        assert_eq!(spoken_took(secs(120)), "2 minutes");
        assert_eq!(spoken_took(secs(2 * 3600 + 3 * 60)), "2 hours 3 minutes");
    }

    #[test]
    fn panel_tabs_read_as_voiceover_says() {
        assert_eq!(panel_tab(PanelTab::Changes { files: Some(3) }), "Changes, 3 changed files");
        assert_eq!(panel_tab(PanelTab::Changes { files: Some(1) }), "Changes, 1 changed file");
        assert_eq!(panel_tab(PanelTab::Changes { files: Some(0) }), "Changes, no changes");
        assert_eq!(panel_tab(PanelTab::Changes { files: None }), "Changes", "not known, or no repository");
        assert_eq!(panel_tab(PanelTab::History), "History");
        assert_eq!(panel_tab(PanelTab::Project), "Project");
        assert_eq!(panel_tab(PanelTab::Usage), "Usage");
    }

    #[test]
    fn the_notes_tab_says_what_is_open_and_leaves_zeros_out() {
        assert_eq!(
            panel_tab(PanelTab::Notes { open_steps: 2, notes: 1 }),
            "Notes, 2 open next steps, 1 note"
        );
        assert_eq!(panel_tab(PanelTab::Notes { open_steps: 1, notes: 0 }), "Notes, 1 open next step");
        assert_eq!(panel_tab(PanelTab::Notes { open_steps: 0, notes: 3 }), "Notes, 3 notes");
        assert_eq!(panel_tab(PanelTab::Notes { open_steps: 0, notes: 0 }), "Notes");
    }

    #[test]
    fn section_bands_say_their_count_in_words() {
        assert_eq!(section_band("Next steps", 2, Counted::Open), "Next steps, 2 open");
        assert_eq!(section_band("Next steps", 0, Counted::Open), "Next steps, none open");
        assert_eq!(section_band("Notes", 2, Counted::Notes), "Notes, 2 notes");
        assert_eq!(section_band("Notes", 1, Counted::Notes), "Notes, 1 note");
        assert_eq!(section_band("Notes", 0, Counted::Notes), "Notes, no notes");
        assert_eq!(section_band("Changes", 3, Counted::Files), "Changes, 3 files");
        assert_eq!(section_band("Untracked", 1, Counted::Files), "Untracked, 1 file");
        assert_eq!(section_band("Staged", 1_102, Counted::Files), "Staged, 1102 files");
        assert_eq!(section_band("Commits", 1, Counted::Commits), "Commits, 1 commit");
        assert_eq!(section_band("Actions", 4, Counted::Items), "Actions, 4 items");
        assert_eq!(section_band("Prompts", 0, Counted::Items), "Prompts, no items");
    }

    fn entry(
        activity: Activity,
        since: Instant,
        agent: Option<&'static str>,
        name: &'static str,
    ) -> Entry<'static> {
        Entry { activity, since, agent, name }
    }

    #[test]
    fn the_agent_items_are_spoken_whole_at_any_width() {
        let now = Instant::now();
        let ago = |secs| now - Duration::from_secs(secs);
        let entries = [
            entry(Activity::Working, ago(4 * 60 + 10), Some("Claude"), "sidebar polish"),
            entry(Activity::NeedsYou, ago(30), Some("Claude"), "diff perf"),
            entry(Activity::Quiet, ago(5), None, "zsh"),
            entry(Activity::Done, ago(12 * 60), Some("Codex"), "auth tokens"),
        ];
        let summary = Summary::of(&entries);
        assert_eq!(status_working(&summary, &entries, now).as_deref(), Some("Claude working, 4 minutes"));
        assert_eq!(status_needs_you(&summary).as_deref(), Some("1 terminal needs you"));
        assert_eq!(status_done(&summary).as_deref(), Some("1 terminal finished"));
        // A narrow window shows only "4m" and "1"; speech keeps every word.
        assert_eq!(summary.working(&entries, Detail::Lean, now).as_deref(), Some("4m"));
        assert_eq!(summary.done(Detail::Lean).as_deref(), Some("1"));
    }

    #[test]
    fn several_terminals_are_counted() {
        let now = Instant::now();
        let entries = [
            entry(Activity::Working, now, Some("Claude"), "a"),
            entry(Activity::Working, now, None, "b"),
            entry(Activity::NeedsYou, now, None, "c"),
            entry(Activity::NeedsYou, now, Some("Codex"), "d"),
            entry(Activity::Done, now, None, "e"),
            entry(Activity::Done, now, None, "f"),
            entry(Activity::Done, now, None, "g"),
            entry(Activity::Failed, now, None, "h"),
            entry(Activity::Failed, now, Some("brew"), "i"),
        ];
        let summary = Summary::of(&entries);
        assert_eq!(status_working(&summary, &entries, now).as_deref(), Some("2 terminals working"));
        assert_eq!(status_needs_you(&summary).as_deref(), Some("2 terminals need you"));
        assert_eq!(status_done(&summary).as_deref(), Some("3 terminals finished"));
        assert_eq!(status_failed(&summary).as_deref(), Some("2 terminals failed"));
        let one_failed = Summary::of(&entries[7..8]);
        assert_eq!(status_failed(&one_failed).as_deref(), Some("1 terminal failed"));
        // A command without an agent says it is one terminal.
        let one = [entry(Activity::Working, now - Duration::from_secs(9), None, "cargo test")];
        assert_eq!(
            status_working(&Summary::of(&one), &one, now).as_deref(),
            Some("1 terminal working, less than a minute")
        );
    }

    #[test]
    fn nothing_to_count_says_nothing() {
        let now = Instant::now();
        let entries = [entry(Activity::Quiet, now, None, "zsh")];
        let summary = Summary::of(&entries);
        assert_eq!(status_working(&summary, &entries, now), None);
        assert_eq!(status_needs_you(&summary), None);
        assert_eq!(status_done(&summary), None);
    }

    #[test]
    fn a_pane_says_what_runs_where_it_is_and_whether_it_has_the_keys() {
        assert_eq!(
            super::pane("claude", Some("terminal"), 1, 3, true),
            "claude, terminal, pane 1 of 3, focused"
        );
        assert_eq!(super::pane("cargo test", None, 2, 3, false), "cargo test, pane 2 of 3");
        assert_eq!(super::pane("zsh", Some(""), 1, 2, false), "zsh, pane 1 of 2");
    }

    #[test]
    fn the_branch_and_size_items_read_in_words() {
        assert_eq!(status_branch("ui-unify", 3), "Branch ui-unify, 3 changes");
        assert_eq!(status_branch("ui-unify", 1), "Branch ui-unify, 1 change");
        assert_eq!(status_branch("ui-unify", 0), "Branch ui-unify, no changes");
        assert_eq!(status_branch("HEAD", 2), "Detached HEAD, 2 changes");
        assert_eq!(status_size("zsh", 93, 40), "zsh, 93 columns by 40 rows");
        assert_eq!(status_size("fish", 1, 1), "fish, 1 column by 1 row");
        assert_eq!(status_context(62), "Agent context, 62 percent");
    }

    #[test]
    fn durations_are_spoken_to_the_minute() {
        let secs = Duration::from_secs;
        // As on screen: no seconds.
        assert_eq!(spoken_elapsed(secs(0)), "less than a minute");
        assert_eq!(spoken_elapsed(secs(59)), "less than a minute");
        assert_eq!(spoken_elapsed(secs(60)), "1 minute");
        assert_eq!(spoken_elapsed(secs(4 * 60 + 59)), "4 minutes");
        assert_eq!(spoken_elapsed(secs(3600)), "1 hour");
        assert_eq!(spoken_elapsed(secs(3600 + 5 * 60 + 30)), "1 hour 5 minutes");
        assert_eq!(spoken_elapsed(secs(2 * 3600 + 60)), "2 hours 1 minute");
    }
}
