//! The terminals of every window as the ambient surfaces list them (menu, Dock menu,
//! quick panel): sections by status in the order that matters, the status item's one state
//! (most urgent wins), its count and tooltip, and which terminal ⌥⌘J goes to next.

use std::time::{Duration, Instant};

use super::{Source, State};
use crate::sidebar::text::count;

/// The status item's count stops here ("9+"), so its width stays put.
pub const MENU_BADGE_MAX: usize = 9;

/// A sidebar terminal of any window, as the ambient surfaces show it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Terminal {
    pub session: i64,
    /// The tab that speaks for it (its most urgent), where choosing it goes.
    pub tab: i64,
    pub name: String,
    pub state: State,
    /// When it started doing that.
    pub since: Instant,
    /// What tells its state: an agent, a program that reports its own, or neither.
    pub source: Source,
    /// The name of the agent ("Claude Code") or of the program that reports ("brew"), when
    /// known.
    pub who: Option<String>,
    /// The agent's or program's question, or what it says it is doing.
    pub message: Option<String>,
    /// The command in the foreground, or (finished) the last one that ended by itself.
    pub command: Option<String>,
    /// How long that last command ran.
    pub took: Option<Duration>,
    /// The exit code of the command that failed, while it counts as failed.
    pub exit: Option<i32>,
    /// Files changed in its repository, when known.
    pub changed: Option<usize>,
    pub folder: Option<String>,
    pub branch: Option<String>,
}

impl Terminal {
    /// A terminal that nothing tells the state of, and that needs you, does so for its bell.
    pub fn bell(&self) -> bool {
        self.state == State::NeedsYou && !self.source.told()
    }

    /// The row's second line. The menu leads with the agent or program ("Claude Code · Approve
    /// cargo test?"); the quick panel, whose row is one line, gives just the words ("Approve
    /// cargo test?", "Finished · 7 files changed"). A terminal that nothing tells says its
    /// command, and with it how a failed one went ("cargo build · exit 101 · after 38s").
    pub fn detail(&self, with_agent: bool) -> Option<String> {
        let who = self.who.as_deref().filter(|_| with_agent);
        let changed = self.changed.filter(|&n| n > 0).map(|n| count(n, "file changed", "files changed"));
        let took = self.took.map(|took| format!("after {}", super::duration(took)));
        let exit = self.exit.map(|code| format!("exit {code}"));
        let parts: Vec<Option<&str>> = match (self.state, self.source.told()) {
            (State::NeedsYou | State::Working, true) => vec![who, self.message.as_deref()],
            (State::NeedsYou, false) => vec![Some("Bell"), self.command.as_deref()],
            (State::Working, false) => vec![self.command.as_deref()],
            // A program's own last words ("Upgraded 12 packages") say what finished; an agent's turn
            // ends without any, and says what it changed.
            (State::Done, true) => {
                vec![who.or((!with_agent).then_some("Finished")), self.message.as_deref(), changed.as_deref()]
            }
            (State::Done, false) => match &self.command {
                Some(command) => vec![Some(command.as_str()), took.as_deref()],
                None => vec![Some("Finished")],
            },
            (State::Failed, true) => vec![who, Some("Failed"), self.message.as_deref()],
            (State::Failed, false) => {
                vec![self.command.as_deref().or(Some("Failed")), exit.as_deref(), took.as_deref()]
            }
            (State::Idle, _) => vec![self.who.as_deref().or(self.folder.as_deref())],
            (State::Exited, _) => vec![Some("Exited")],
        };
        let words: Vec<&str> =
            parts.into_iter().flatten().map(str::trim).filter(|part| !part.is_empty()).collect();
        (!words.is_empty()).then(|| words.join(" · "))
    }
}

/// The terminals each section lists, as indices: needing you the longest waiting first, working
/// the longest running first, finished the newest first, and the rest (idle and ended) as given.
/// A terminal that failed is in the finished section, mixed in by when it ended, its row marked as
/// failed: its exit code is a result to look at, like any other (F-40).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sections {
    pub needs_you: Vec<usize>,
    pub working: Vec<usize>,
    pub finished: Vec<usize>,
    pub other: Vec<usize>,
}

pub fn sections(terminals: &[Terminal]) -> Sections {
    let mut sections = Sections::default();
    for (ix, terminal) in terminals.iter().enumerate() {
        match terminal.state {
            State::NeedsYou => sections.needs_you.push(ix),
            State::Working => sections.working.push(ix),
            State::Done | State::Failed => sections.finished.push(ix),
            State::Idle | State::Exited => sections.other.push(ix),
        }
    }
    // Stable sorts: terminals that started at the same moment keep their sidebar order.
    sections.needs_you.sort_by_key(|&ix| terminals[ix].since);
    sections.working.sort_by_key(|&ix| terminals[ix].since);
    sections.finished.sort_by_key(|&ix| std::cmp::Reverse(terminals[ix].since));
    sections
}

/// What the status item shows: the most urgent state of any terminal wins (needs you › failed ›
/// finished › working › idle).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ItemState {
    Idle,
    Working,
    /// A terminal finished and nobody has looked yet.
    Finished,
    /// A command ended with a non-zero exit code, or an agent stopped on an error, and nobody has
    /// looked yet.
    Failed,
    NeedsYou,
}

/// How many terminals are in each state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub needs_you: usize,
    pub working: usize,
    pub finished: usize,
    pub failed: usize,
    /// Idle or ended.
    pub quiet: usize,
}

impl Counts {
    pub fn of(terminals: &[Terminal]) -> Self {
        let mut counts = Counts::default();
        for terminal in terminals {
            match terminal.state {
                State::NeedsYou => counts.needs_you += 1,
                State::Working => counts.working += 1,
                State::Done => counts.finished += 1,
                State::Failed => counts.failed += 1,
                State::Idle | State::Exited => counts.quiet += 1,
            }
        }
        counts
    }

    /// The results nobody has looked at yet, finished or failed: what "needs you + finished" adds
    /// to the Dock badge.
    pub fn results(self) -> usize {
        self.finished + self.failed
    }

    fn total(self) -> usize {
        self.needs_you + self.working + self.results() + self.quiet
    }
}

pub fn item_state(counts: Counts) -> ItemState {
    match counts {
        Counts { needs_you: 1.., .. } => ItemState::NeedsYou,
        Counts { failed: 1.., .. } => ItemState::Failed,
        Counts { finished: 1.., .. } => ItemState::Finished,
        Counts { working: 1.., .. } => ItemState::Working,
        _ => ItemState::Idle,
    }
}

/// The count beside the status item: `None` for none, "9+" past [`MENU_BADGE_MAX`].
pub fn badge(n: usize) -> Option<String> {
    match n {
        0 => None,
        n if n > MENU_BADGE_MAX => Some(format!("{MENU_BADGE_MAX}+")),
        n => Some(n.to_string()),
    }
}

/// The status item's tooltip: "Wormhole — 2 need you · 1 failed · 2 working · 2 finished".
pub fn tooltip(counts: Counts) -> String {
    let total = counts.total();
    let parts: Vec<String> = [
        (counts.needs_you > 0).then(|| match counts.needs_you {
            1 => "1 needs you".to_string(),
            n => format!("{n} need you"),
        }),
        (counts.failed > 0).then(|| format!("{} failed", counts.failed)),
        (counts.working > 0).then(|| format!("{} working", counts.working)),
        (counts.finished > 0).then(|| format!("{} finished", counts.finished)),
    ]
    .into_iter()
    .flatten()
    .collect();
    let summary = match (total, parts.is_empty()) {
        (0, _) => "no terminals open".to_string(),
        (_, true) => format!("{}, all quiet", count(total, "terminal", "terminals")),
        (_, false) => parts.join(" · "),
    };
    format!("Wormhole — {summary}")
}

/// Where ⌥⌘J goes: the terminal that has waited longest; pressed again, the next one
/// (`last` is where it went before, wrapping around). `waiting` is in waiting order (tabs).
pub fn next_waiting(waiting: &[i64], last: Option<i64>) -> Option<i64> {
    let after = last.and_then(|last| waiting.iter().position(|&tab| tab == last)).map_or(0, |ix| ix + 1);
    waiting.get(after % waiting.len().max(1)).copied()
}

/// Whether a terminal is what the quick panel's search asks for: every word is found in its
/// name, folder, branch, command, agent or program, or their words (case doesn't matter).
pub fn matches(query: &str, terminal: &Terminal) -> bool {
    let fields = [
        Some(terminal.name.as_str()),
        terminal.folder.as_deref(),
        terminal.branch.as_deref(),
        terminal.command.as_deref(),
        terminal.who.as_deref(),
        terminal.message.as_deref(),
    ];
    let haystack = fields.into_iter().flatten().collect::<Vec<_>>().join(" ").to_lowercase();
    query.split_whitespace().all(|word| haystack.contains(&word.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{
        Counts, ItemState, Sections, Terminal, badge, item_state, matches, next_waiting, sections, tooltip,
    };
    use crate::notify::{Source, State};

    fn terminal(tab: i64, state: State, since: Instant) -> Terminal {
        Terminal {
            session: tab,
            tab,
            name: format!("t{tab}"),
            state,
            since,
            source: Source::Plain,
            who: None,
            message: None,
            command: None,
            took: None,
            exit: None,
            changed: None,
            folder: None,
            branch: None,
        }
    }

    fn ago(t0: Instant, secs: u64) -> Instant {
        t0 - Duration::from_secs(secs)
    }

    #[test]
    fn sections_sort_by_urgency_and_age() {
        let t0 = Instant::now();
        let terminals = [
            terminal(1, State::Idle, t0),
            terminal(2, State::NeedsYou, ago(t0, 120)),
            terminal(3, State::NeedsYou, ago(t0, 300)),
            terminal(4, State::Working, ago(t0, 72)),
            terminal(5, State::Working, ago(t0, 252)),
            terminal(6, State::Done, ago(t0, 720)),
            terminal(7, State::Done, ago(t0, 120)),
            terminal(8, State::Exited, t0),
        ];
        assert_eq!(
            sections(&terminals),
            Sections { needs_you: vec![2, 1], working: vec![4, 3], finished: vec![6, 5], other: vec![0, 7] },
            "longest waiting, longest running, newest finished first"
        );
    }

    #[test]
    fn a_failed_terminal_is_listed_among_the_finished_by_when_it_ended() {
        let t0 = Instant::now();
        let terminals = [
            terminal(1, State::Done, ago(t0, 720)),
            terminal(2, State::Failed, ago(t0, 120)),
            terminal(3, State::Done, ago(t0, 300)),
        ];
        assert_eq!(sections(&terminals).finished, [1, 2, 0]);
    }

    #[test]
    fn the_most_urgent_state_wins_the_status_item() {
        let t0 = Instant::now();
        let all = |states: &[State]| {
            let terminals: Vec<_> =
                states.iter().enumerate().map(|(i, s)| terminal(i as i64, *s, t0)).collect();
            item_state(Counts::of(&terminals))
        };
        assert_eq!(all(&[]), ItemState::Idle);
        assert_eq!(all(&[State::Idle, State::Exited]), ItemState::Idle);
        assert_eq!(all(&[State::Idle, State::Working]), ItemState::Working);
        assert_eq!(all(&[State::Working, State::Done]), ItemState::Finished);
        assert_eq!(all(&[State::Done, State::NeedsYou, State::Working]), ItemState::NeedsYou);
        // Failed outranks finished and working, and needing you outranks failed.
        assert_eq!(all(&[State::Working, State::Failed, State::Done]), ItemState::Failed);
        assert_eq!(all(&[State::Failed, State::NeedsYou]), ItemState::NeedsYou);
        assert_eq!(all(&[State::Idle, State::Failed]), ItemState::Failed);
    }

    #[test]
    fn the_count_caps_at_nine() {
        assert_eq!(badge(0), None);
        assert_eq!(badge(2).as_deref(), Some("2"));
        assert_eq!(badge(9).as_deref(), Some("9"));
        assert_eq!(badge(12).as_deref(), Some("9+"));
    }

    #[test]
    fn the_tooltip_sums_up_most_urgent_first() {
        let counts = Counts { needs_you: 2, working: 2, finished: 2, failed: 0, quiet: 1 };
        assert_eq!(tooltip(counts), "Wormhole — 2 need you · 2 working · 2 finished");
        let failed = Counts { failed: 1, ..counts };
        assert_eq!(tooltip(failed), "Wormhole — 2 need you · 1 failed · 2 working · 2 finished");
        assert_eq!(tooltip(Counts { failed: 3, ..Counts::default() }), "Wormhole — 3 failed");
        assert_eq!((failed.results(), counts.results()), (3, 2), "the Dock badge counts both");
        assert_eq!(tooltip(Counts { needs_you: 1, ..Counts::default() }), "Wormhole — 1 needs you");
        assert_eq!(tooltip(Counts { quiet: 3, ..Counts::default() }), "Wormhole — 3 terminals, all quiet");
        assert_eq!(tooltip(Counts { quiet: 1, ..Counts::default() }), "Wormhole — 1 terminal, all quiet");
        assert_eq!(tooltip(Counts::default()), "Wormhole — no terminals open");
    }

    #[test]
    fn jumping_goes_longest_waiting_first_then_the_next() {
        let waiting = [5, 9, 2];
        assert_eq!(next_waiting(&waiting, None), Some(5));
        assert_eq!(next_waiting(&waiting, Some(5)), Some(9));
        assert_eq!(next_waiting(&waiting, Some(2)), Some(5), "wraps around");
        assert_eq!(next_waiting(&waiting, Some(7)), Some(5), "the last one isn't waiting any more");
        assert_eq!(next_waiting(&[], Some(5)), None, "nothing waiting: nothing happens");
    }

    #[test]
    fn rows_say_who_and_what() {
        let t0 = Instant::now();
        let mut agent = terminal(1, State::NeedsYou, t0);
        agent.source = Source::Agent;
        agent.who = Some("Claude Code".into());
        agent.message = Some("Approve cargo test?".into());
        assert_eq!(agent.detail(true).as_deref(), Some("Claude Code · Approve cargo test?"));
        assert_eq!(agent.detail(false).as_deref(), Some("Approve cargo test?"));
        agent.state = State::Done;
        agent.message = None; // a turn ends without words
        agent.changed = Some(7);
        assert_eq!(agent.detail(true).as_deref(), Some("Claude Code · 7 files changed"));
        assert_eq!(agent.detail(false).as_deref(), Some("Finished · 7 files changed"));
        agent.state = State::Working;
        agent.message = None;
        assert_eq!(agent.detail(true).as_deref(), Some("Claude Code"));
        assert_eq!(agent.detail(false), None);
        agent.state = State::Failed;
        agent.message = Some("Rate limit reached".into());
        assert_eq!(agent.detail(true).as_deref(), Some("Claude Code · Failed · Rate limit reached"));
        assert_eq!(agent.detail(false).as_deref(), Some("Failed · Rate limit reached"));

        // A program that reports its own status reads like an agent, under its own name, and its
        // bell-less "needs you" is a question.
        let mut brew = terminal(3, State::NeedsYou, t0);
        brew.source = Source::Program;
        brew.who = Some("brew".into());
        brew.message = Some("Password:".into());
        assert!(!brew.bell());
        assert_eq!(brew.detail(true).as_deref(), Some("brew · Password:"));
        brew.state = State::Done;
        assert_eq!(brew.detail(true).as_deref(), Some("brew · Password:"));
        brew.message = Some("Upgraded 12 packages".into());
        assert_eq!(brew.detail(true).as_deref(), Some("brew · Upgraded 12 packages"));
        assert_eq!(brew.detail(false).as_deref(), Some("Finished · Upgraded 12 packages"));
        brew.message = None;
        assert_eq!(brew.detail(true).as_deref(), Some("brew"));
        assert_eq!(brew.detail(false).as_deref(), Some("Finished"));

        let mut plain = terminal(2, State::NeedsYou, t0);
        plain.command = Some("ssh deploy@prod-1".into());
        assert!(plain.bell());
        assert_eq!(plain.detail(true).as_deref(), Some("Bell · ssh deploy@prod-1"));
        plain.state = State::Working;
        plain.command = Some("cargo test -p wormhole-core".into());
        assert_eq!(plain.detail(false).as_deref(), Some("cargo test -p wormhole-core"));
        plain.state = State::Done;
        plain.command = Some("cargo build".into());
        plain.took = Some(Duration::from_secs(38));
        assert_eq!(plain.detail(true).as_deref(), Some("cargo build · after 38s"));
        plain.command = None;
        assert_eq!(plain.detail(true).as_deref(), Some("Finished"));
        // A failed command says its exit code and how long it ran (F-40).
        plain.state = State::Failed;
        plain.command = Some("cargo build".into());
        plain.exit = Some(101);
        assert_eq!(plain.detail(true).as_deref(), Some("cargo build · exit 101 · after 38s"));
        plain.command = None;
        plain.took = None;
        assert_eq!(plain.detail(true).as_deref(), Some("Failed · exit 101"));
        plain.state = State::Idle;
        plain.folder = Some("dotfiles".into());
        assert_eq!(plain.detail(true).as_deref(), Some("dotfiles"));
        plain.state = State::Exited;
        assert_eq!(plain.detail(true).as_deref(), Some("Exited"));
    }

    #[test]
    fn search_finds_terminals_by_any_of_their_words() {
        let mut t = terminal(1, State::Working, Instant::now());
        t.name = "diff perf".into();
        t.folder = Some("terminal".into());
        t.branch = Some("ui-unify".into());
        t.command = Some("cargo test".into());
        assert!(matches("", &t));
        assert!(matches("DIFF", &t));
        assert!(matches("unify cargo", &t), "any order, across fields");
        assert!(!matches("diff zebra", &t), "every word must be found");
    }
}
