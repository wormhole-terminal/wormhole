//! The status bar: optional and hidden by default (`status_bar` in config.json). Along the
//! bottom of a window it shows the agents of every window by what they are doing and the
//! terminal's size (the branch and the agent's context are the title row's). This is what it
//! says, and how much of it fits a window's width.

use std::time::{Duration, Instant};

use crate::view::limits::{STATUS_BAR_SIZE_MIN, STATUS_BAR_WORDS_MIN};
use crate::view::sidebar::wording::UNDER_A_MINUTE;

/// The bar's height. The panes above it end there while it is shown.
pub const STATUS_BAR_H: f32 = 28.0;

/// How much the bar says at a window's width.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Detail {
    /// Below [`STATUS_BAR_SIZE_MIN`]: numbers, and no terminal size.
    Lean,
    /// Below [`STATUS_BAR_WORDS_MIN`]: the items drop their words and keep their numbers: the
    /// button says how long it has waited ("2m"), the rest are a glyph and a count.
    Numbers,
    Words,
}

impl Detail {
    pub fn at(window_width: f32) -> Self {
        if window_width < STATUS_BAR_SIZE_MIN {
            Detail::Lean
        } else if window_width < STATUS_BAR_WORDS_MIN {
            Detail::Numbers
        } else {
            Detail::Words
        }
    }

    pub fn words(self) -> bool {
        self == Detail::Words
    }

    /// Whether the terminal's size is shown.
    pub fn size(self) -> bool {
        self >= Detail::Numbers
    }

    /// Whether the profile's name is shown (a window in another profile than the shown one). It is
    /// the bar's lowest-priority item: it goes first, with the words, when the window narrows.
    pub fn profile(self) -> bool {
        self.words()
    }
}

/// How tall the panes are in a window this tall: the bar, while shown, takes its height from
/// them; hidden, it gives it back.
pub fn pane_height(window_height: f32, shown: bool) -> f32 {
    if shown { (window_height - STATUS_BAR_H).max(0.0) } else { window_height }
}

/// What a terminal is doing, as far as the bar counts it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Working,
    NeedsYou,
    Done,
    /// Ended on an error (a non-zero exit code, an agent or program that failed) and not seen.
    Failed,
    /// Idle or exited: not counted.
    Quiet,
}

/// One terminal (of any window) as the bar sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Entry<'a> {
    pub activity: Activity,
    /// When it started doing that.
    pub since: Instant,
    /// The agent's name ("Claude Code"), or a reporting program's ("brew"), if one runs there.
    pub agent: Option<&'a str>,
    /// The terminal's name, for the needs-you item's tooltip.
    pub name: &'a str,
}

/// One count after the attention button: how many terminals are doing `activity`, and the first
/// of them (an index into the entries), which a click goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Counted {
    pub activity: Activity,
    pub count: usize,
    pub terminal: usize,
}

impl Counted {
    /// "2 working", "1 done", "3 failed", "2 need input": all of them in words.
    pub fn words(&self) -> String {
        let n = self.count;
        match self.activity {
            Activity::Working => format!("{n} working"),
            Activity::NeedsYou if n == 1 => "1 needs input".to_string(),
            Activity::NeedsYou => format!("{n} need input"),
            Activity::Done => format!("{n} done"),
            Activity::Failed => format!("{n} failed"),
            Activity::Quiet => n.to_string(),
        }
    }
}

/// The counts as one muted line: "2 working · 1 done".
pub fn rest_line(rest: &[Counted]) -> String {
    rest.iter().map(Counted::words).collect::<Vec<_>>().join(" · ")
}

/// The terminals each agent item counts, as indices into the entries: working, done and failed in
/// the order given (each item opens the first), needing you the longest waiting first (that item
/// opens the one that has waited longest).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Summary {
    pub working: Vec<usize>,
    pub needs_you: Vec<usize>,
    pub done: Vec<usize>,
    pub failed: Vec<usize>,
}

impl Summary {
    pub fn of(entries: &[Entry]) -> Self {
        let mut summary = Summary::default();
        for (ix, entry) in entries.iter().enumerate() {
            match entry.activity {
                Activity::Working => summary.working.push(ix),
                Activity::NeedsYou => summary.needs_you.push(ix),
                Activity::Done => summary.done.push(ix),
                Activity::Failed => summary.failed.push(ix),
                Activity::Quiet => {}
            }
        }
        // Stable, so terminals that started waiting at the same moment keep their order.
        summary.needs_you.sort_by_key(|&ix| entries[ix].since);
        summary
    }

    /// The terminal the bar's button is for: the one that has waited longest for you (an index
    /// into the entries). `None` while nothing needs input.
    pub fn attention(&self) -> Option<usize> {
        self.needs_you.first().copied()
    }

    /// What the attention button says after the terminal's name: "needs input · 2m", and only
    /// "2m" once the words have gone.
    pub fn attention_words(&self, entries: &[Entry], detail: Detail, now: Instant) -> Option<String> {
        let entry = &entries[self.attention()?];
        let waited = elapsed(now.saturating_duration_since(entry.since));
        Some(if detail.words() { format!("needs input · {waited}") } else { waited })
    }

    /// The rest, as the bar counts them after the button: any others that need input, then what
    /// works, what is done and what failed, each with how many and the terminal a click goes to.
    pub fn rest(&self) -> Vec<Counted> {
        let others = self.needs_you.get(1..).unwrap_or_default();
        [
            (Activity::NeedsYou, others),
            (Activity::Working, &self.working[..]),
            (Activity::Done, &self.done[..]),
            (Activity::Failed, &self.failed[..]),
        ]
        .into_iter()
        .filter_map(|(activity, group)| {
            group.first().map(|&terminal| Counted { activity, count: group.len(), terminal })
        })
        .collect()
    }

    /// Who needs input, longest waiting first, and what a click does ("diff perf, then prod · go
    /// to the first").
    pub fn needs_you_tooltip(&self, entries: &[Entry]) -> Option<String> {
        const NAMED: usize = 3;
        let names: Vec<&str> = self.needs_you.iter().take(NAMED).map(|&ix| entries[ix].name).collect();
        let (first, rest) = names.split_first()?;
        let mut text = (*first).to_string();
        if !rest.is_empty() {
            text.push_str(", then ");
            text.push_str(&rest.join(", "));
        }
        if let Some(more) = self.needs_you.len().checked_sub(NAMED).filter(|&n| n > 0) {
            text.push_str(&format!(" and {more} more"));
        }
        text.push_str(if rest.is_empty() { " · go there" } else { " · go to the first" });
        Some(text)
    }
}

/// How long something has gone on, in whole minutes: "<1m", "4m", "1h 5m".
/// Never seconds: a clock that ticks every second pulls the eye and redraws the bar for nothing.
pub fn elapsed(time: Duration) -> String {
    let secs = time.as_secs();
    match secs {
        0..60 => UNDER_A_MINUTE.to_string(),
        60..3600 => format!("{}m", secs / 60),
        _ => format!("{}h {}m", secs / 3600, secs % 3600 / 60),
    }
}

/// The profile item's tooltip: what the name at the bar's left means.
pub fn profile_tooltip(name: &str) -> String {
    format!("This window is in {name}, not the profile Wormhole is in")
}

/// The terminal's size item: its shell and its grid ("zsh · 90×40").
pub fn size(shell: &str, cols: usize, rows: usize) -> String {
    format!("{shell} · {cols}×{rows}")
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        Activity, Counted, Detail, Entry, STATUS_BAR_H, Summary, elapsed, pane_height, rest_line, size,
    };

    #[test]
    fn the_bar_sheds_detail_as_the_window_narrows() {
        // Below 1100 the words drop, below 720 the size goes too.
        assert_eq!(Detail::at(1440.0), Detail::Words);
        assert_eq!(Detail::at(1100.0), Detail::Words);
        assert_eq!(Detail::at(1099.0), Detail::Numbers);
        assert_eq!(Detail::at(900.0), Detail::Numbers);
        assert_eq!(Detail::at(720.0), Detail::Numbers);
        assert_eq!(Detail::at(719.0), Detail::Lean);
        assert_eq!(Detail::at(670.0), Detail::Lean);
        assert!(Detail::Words.words() && !Detail::Numbers.words() && !Detail::Lean.words());
        assert!(Detail::Words.size() && Detail::Numbers.size() && !Detail::Lean.size());
        // The profile's name is the first to go.
        assert!(Detail::Words.profile() && !Detail::Numbers.profile() && !Detail::Lean.profile());
    }

    #[test]
    fn the_panes_end_above_the_bar_only_while_it_is_shown() {
        assert_eq!(STATUS_BAR_H, 28.0);
        assert_eq!(pane_height(900.0, true), 872.0);
        assert_eq!(pane_height(900.0, false), 900.0);
        assert_eq!(pane_height(10.0, true), 0.0);
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
    fn the_longest_waiting_terminal_is_the_button() {
        let now = Instant::now();
        let ago = |secs| now - Duration::from_secs(secs);
        let entries = [
            entry(Activity::NeedsYou, ago(60), None, "prod"),
            entry(Activity::Working, ago(10), None, "build"),
            entry(Activity::NeedsYou, ago(2 * 60 + 5), Some("Claude"), "diff perf"),
            entry(Activity::Quiet, ago(5), None, "zsh"),
        ];
        let summary = Summary::of(&entries);
        assert_eq!(summary.attention(), Some(2), "waiting 2m, not the one waiting 1m");
        assert_eq!(
            summary.attention_words(&entries, Detail::Words, now).as_deref(),
            Some("needs input · 2m")
        );
        // Under ~1100 the words go first: only how long it has waited stays.
        assert_eq!(summary.attention_words(&entries, Detail::Numbers, now).as_deref(), Some("2m"));
        assert_eq!(summary.attention_words(&entries, Detail::Lean, now).as_deref(), Some("2m"));
        // The other one that needs input is counted with the rest, a click goes to it.
        assert_eq!(
            summary.rest(),
            [
                Counted { activity: Activity::NeedsYou, count: 1, terminal: 0 },
                Counted { activity: Activity::Working, count: 1, terminal: 1 },
            ]
        );
    }

    #[test]
    fn the_same_wait_keeps_the_order_of_the_terminals() {
        let now = Instant::now();
        let entries = [entry(Activity::NeedsYou, now, None, "a"), entry(Activity::NeedsYou, now, None, "b")];
        assert_eq!(Summary::of(&entries).attention(), Some(0));
    }

    #[test]
    fn the_rest_is_numbers_in_a_line() {
        let now = Instant::now();
        let entries = [
            entry(Activity::Working, now, Some("Claude"), "sidebar polish"),
            entry(Activity::Working, now, None, "cargo test"),
            entry(Activity::NeedsYou, now, Some("Claude"), "diff perf"),
            entry(Activity::Done, now, Some("Codex"), "auth tokens"),
            entry(Activity::Failed, now, None, "build"),
            entry(Activity::Failed, now, Some("brew"), "upgrade"),
        ];
        let summary = Summary::of(&entries);
        assert_eq!(summary.attention(), Some(2));
        let rest = summary.rest();
        // The terminal in the button isn't counted again.
        assert_eq!(rest_line(&rest), "2 working · 1 done · 2 failed");
        assert_eq!(rest.iter().map(|c| c.terminal).collect::<Vec<_>>(), [0, 3, 4], "each goes to its first");
        assert_eq!(Counted { activity: Activity::NeedsYou, count: 1, terminal: 0 }.words(), "1 needs input");
        assert_eq!(Counted { activity: Activity::NeedsYou, count: 3, terminal: 0 }.words(), "3 need input");
    }

    #[test]
    fn nothing_to_count_shows_no_item() {
        let now = Instant::now();
        let entries = [entry(Activity::Quiet, now, None, "zsh")];
        let summary = Summary::of(&entries);
        assert_eq!(summary, Summary::default());
        assert_eq!(summary.attention(), None);
        assert_eq!(summary.attention_words(&entries, Detail::Words, now), None);
        assert!(summary.rest().is_empty());
        assert_eq!(rest_line(&summary.rest()), "");
        assert_eq!(summary.needs_you_tooltip(&entries), None);
    }

    #[test]
    fn a_failed_terminal_is_counted_not_a_button() {
        let now = Instant::now();
        let entries = [
            entry(Activity::Failed, now, None, "build"),
            entry(Activity::Done, now, Some("Codex"), "auth tokens"),
            entry(Activity::Failed, now, Some("brew"), "upgrade"),
        ];
        let summary = Summary::of(&entries);
        assert_eq!((summary.failed.as_slice(), summary.done.as_slice()), (&[0, 2][..], &[1][..]));
        assert_eq!(summary.attention(), None);
        assert_eq!(rest_line(&summary.rest()), "1 done · 2 failed");
    }

    #[test]
    fn the_tooltip_names_who_waits_longest_first() {
        let now = Instant::now();
        let ago = |secs| now - Duration::from_secs(secs);
        let entries = [
            entry(Activity::NeedsYou, ago(60), None, "prod"),
            entry(Activity::Working, ago(10), None, "build"),
            entry(Activity::NeedsYou, ago(120), Some("Claude"), "diff perf"),
        ];
        let summary = Summary::of(&entries);
        assert_eq!(summary.needs_you, [2, 0]);
        assert_eq!(
            summary.needs_you_tooltip(&entries).as_deref(),
            Some("diff perf, then prod · go to the first")
        );
        let one = [entry(Activity::NeedsYou, now, None, "prod")];
        assert_eq!(Summary::of(&one).needs_you_tooltip(&one).as_deref(), Some("prod · go there"));
        let many: Vec<Entry> = ["a", "b", "c", "d", "e"]
            .into_iter()
            .enumerate()
            .map(|(ix, name)| entry(Activity::NeedsYou, ago(100 - ix as u64), None, name))
            .collect();
        assert_eq!(
            Summary::of(&many).needs_you_tooltip(&many).as_deref(),
            Some("a, then b, c and 2 more · go to the first")
        );
    }

    #[test]
    fn times_count_whole_minutes() {
        // No seconds, so nothing ticks in the first minute.
        assert_eq!(elapsed(Duration::from_secs(0)), "<1m");
        assert_eq!(elapsed(Duration::from_secs(59)), "<1m");
        assert_eq!(elapsed(Duration::from_secs(60)), "1m");
        assert_eq!(elapsed(Duration::from_secs(4 * 60 + 59)), "4m");
        assert_eq!(elapsed(Duration::from_secs(3600 + 5 * 60)), "1h 5m");
    }

    #[test]
    fn the_size_item_names_the_shell_and_the_grid() {
        assert_eq!(size("zsh", 90, 40), "zsh · 90×40");
    }
}
