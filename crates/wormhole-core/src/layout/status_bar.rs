//! The status bar: optional and hidden by default (`status_bar` in config.json). Along the
//! bottom of a window it shows the branch, the agents of every window by what they are doing,
//! how full the selected terminal's agent context window is, and the terminal's size. This is
//! what it says, and how much of it fits a window's width.

use std::time::{Duration, Instant};

use crate::limits::{STATUS_BAR_SIZE_MIN, STATUS_BAR_WORDS_MIN};
use crate::sidebar::text::UNDER_A_MINUTE;

/// The bar's height. The panes above it end there while it is shown.
pub const STATUS_BAR_H: f32 = 28.0;

/// The context meter's track in a wide window, and below [`STATUS_BAR_WORDS_MIN`].
pub const CONTEXT_METER_W: f32 = 48.0;
pub const CONTEXT_METER_W_NARROW: f32 = 32.0;

/// How much the bar says at a window's width.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Detail {
    /// Below [`STATUS_BAR_SIZE_MIN`]: numbers, and no terminal size.
    Lean,
    /// Below [`STATUS_BAR_WORDS_MIN`]: the items drop their words and keep their numbers
    /// ("4m · 1 needs you · 1"); only "needs you" keeps its words.
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

    /// How wide the context meter's track is, `None` where it goes with the size.
    pub fn context_meter(self) -> Option<f32> {
        match self {
            Detail::Words => Some(CONTEXT_METER_W),
            Detail::Numbers => Some(CONTEXT_METER_W_NARROW),
            Detail::Lean => None,
        }
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

    /// The working item: one terminal says who works and for how long ("Claude Code working ·
    /// 4m", only "4m" when narrow); several say how many ("2 working", "2").
    pub fn working(&self, entries: &[Entry], detail: Detail, now: Instant) -> Option<String> {
        match self.working.as_slice() {
            [] => None,
            [one] => {
                let entry = &entries[*one];
                let time = elapsed(now.saturating_duration_since(entry.since));
                Some(match (detail.words(), entry.agent) {
                    (false, _) => time,
                    (true, Some(agent)) => format!("{agent} working · {time}"),
                    (true, None) => format!("1 working · {time}"),
                })
            }
            many if detail.words() => Some(format!("{} working", many.len())),
            many => Some(many.len().to_string()),
        }
    }

    /// The needs-you item, the bar's one loud item: it always keeps its words.
    pub fn needs_you(&self) -> Option<String> {
        match self.needs_you.len() {
            0 => None,
            1 => Some("1 needs you".into()),
            n => Some(format!("{n} need you")),
        }
    }

    /// Who needs you, longest waiting first, and what a click does ("diff perf, then prod · go
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

    /// The done item: "1 done", only "1" when narrow.
    pub fn done(&self, detail: Detail) -> Option<String> {
        match self.done.len() {
            0 => None,
            n if detail.words() => Some(format!("{n} done")),
            n => Some(n.to_string()),
        }
    }

    /// The failed item: "1 failed", only "1" when narrow.
    pub fn failed(&self, detail: Detail) -> Option<String> {
        match self.failed.len() {
            0 => None,
            n if detail.words() => Some(format!("{n} failed")),
            n => Some(n.to_string()),
        }
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

/// The context item's tooltip: "Context: 62% used".
pub fn context_tooltip(percent: u32) -> String {
    format!("Context: {percent}% used")
}

/// The terminal's size item: its shell and its grid ("zsh · 90×40").
pub fn size(shell: &str, cols: usize, rows: usize) -> String {
    format!("{shell} · {cols}×{rows}")
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        Activity, CONTEXT_METER_W, CONTEXT_METER_W_NARROW, Detail, Entry, STATUS_BAR_H, Summary,
        context_tooltip, elapsed, pane_height, size,
    };

    #[test]
    fn the_bar_sheds_detail_as_the_window_narrows() {
        // Below 900 the words drop, below 720 the size goes too.
        assert_eq!(Detail::at(1440.0), Detail::Words);
        assert_eq!(Detail::at(900.0), Detail::Words);
        assert_eq!(Detail::at(899.0), Detail::Numbers);
        assert_eq!(Detail::at(720.0), Detail::Numbers);
        assert_eq!(Detail::at(719.0), Detail::Lean);
        assert_eq!(Detail::at(670.0), Detail::Lean);
        assert!(Detail::Words.words() && !Detail::Numbers.words() && !Detail::Lean.words());
        assert!(Detail::Words.size() && Detail::Numbers.size() && !Detail::Lean.size());
    }

    #[test]
    fn the_context_meter_narrows_then_goes_with_the_size() {
        assert_eq!(Detail::Words.context_meter(), Some(CONTEXT_METER_W));
        assert_eq!(Detail::Numbers.context_meter(), Some(CONTEXT_METER_W_NARROW));
        assert_eq!(Detail::Lean.context_meter(), None);
        assert_eq!((CONTEXT_METER_W, CONTEXT_METER_W_NARROW), (48.0, 32.0));
        assert_eq!(context_tooltip(62), "Context: 62% used");
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
    fn the_agent_items_say_what_the_agent_is_doing() {
        let now = Instant::now();
        let ago = |secs| now - Duration::from_secs(secs);
        let entries = [
            entry(Activity::Working, ago(4 * 60 + 10), Some("Claude"), "sidebar polish"),
            entry(Activity::NeedsYou, ago(30), Some("Claude"), "diff perf"),
            entry(Activity::Quiet, ago(5), None, "zsh"),
            entry(Activity::Done, ago(12 * 60), Some("Codex"), "auth tokens"),
        ];
        let summary = Summary::of(&entries);
        assert_eq!(summary.working(&entries, Detail::Words, now).as_deref(), Some("Claude working · 4m"));
        assert_eq!(summary.needs_you().as_deref(), Some("1 needs you"));
        assert_eq!(summary.done(Detail::Words).as_deref(), Some("1 done"));
        // Below 900: "4m · 1 needs you · 1".
        assert_eq!(summary.working(&entries, Detail::Numbers, now).as_deref(), Some("4m"));
        assert_eq!(summary.done(Detail::Numbers).as_deref(), Some("1"));
        assert_eq!(summary.needs_you_tooltip(&entries).as_deref(), Some("diff perf · go there"));
    }

    #[test]
    fn several_working_say_how_many() {
        let now = Instant::now();
        let entries =
            [entry(Activity::Working, now, Some("Claude"), "a"), entry(Activity::Working, now, None, "b")];
        let summary = Summary::of(&entries);
        assert_eq!(summary.working(&entries, Detail::Words, now).as_deref(), Some("2 working"));
        assert_eq!(summary.working(&entries, Detail::Lean, now).as_deref(), Some("2"));
        // A command without an agent says it is one.
        let one = [entry(Activity::Working, now - Duration::from_secs(9), None, "cargo test")];
        assert_eq!(Summary::of(&one).working(&one, Detail::Words, now).as_deref(), Some("1 working · <1m"));
    }

    #[test]
    fn nothing_to_count_shows_no_item() {
        let now = Instant::now();
        let entries = [entry(Activity::Quiet, now, None, "zsh")];
        let summary = Summary::of(&entries);
        assert_eq!(summary, Summary::default());
        assert_eq!(summary.working(&entries, Detail::Words, now), None);
        assert_eq!(summary.needs_you(), None);
        assert_eq!(summary.done(Detail::Words), None);
        assert_eq!(summary.failed(Detail::Words), None);
        assert_eq!(summary.needs_you_tooltip(&entries), None);
    }

    #[test]
    fn a_failed_terminal_has_its_own_item() {
        let now = Instant::now();
        let entries = [
            entry(Activity::Failed, now, None, "build"),
            entry(Activity::Done, now, Some("Codex"), "auth tokens"),
            entry(Activity::Failed, now, Some("brew"), "upgrade"),
        ];
        let summary = Summary::of(&entries);
        assert_eq!((summary.failed.as_slice(), summary.done.as_slice()), (&[0, 2][..], &[1][..]));
        assert_eq!(summary.failed(Detail::Words).as_deref(), Some("2 failed"));
        assert_eq!(summary.failed(Detail::Numbers).as_deref(), Some("2"));
        assert_eq!(summary.done(Detail::Words).as_deref(), Some("1 done"), "not counted as done");
    }

    #[test]
    fn the_longest_waiting_comes_first() {
        // "2 need you", the tooltip names them longest waiting first, a click goes there.
        let now = Instant::now();
        let ago = |secs| now - Duration::from_secs(secs);
        let entries = [
            entry(Activity::NeedsYou, ago(60), None, "prod"),
            entry(Activity::Working, ago(10), None, "build"),
            entry(Activity::NeedsYou, ago(120), Some("Claude"), "diff perf"),
        ];
        let summary = Summary::of(&entries);
        assert_eq!(summary.needs_you, [2, 0]);
        assert_eq!(summary.needs_you().as_deref(), Some("2 need you"));
        assert_eq!(
            summary.needs_you_tooltip(&entries).as_deref(),
            Some("diff perf, then prod · go to the first")
        );
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
