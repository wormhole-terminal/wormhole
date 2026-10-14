//! What VoiceOver announces when a terminal changes. GPUI has no live regions, so the app
//! posts these as NSAccessibility announcements: a terminal that needs you interrupts what
//! VoiceOver is saying, one that finished or failed waits its turn, and a burst is said as one
//! count, as the notifications' summary groups it.
//!
//! Working never announces: it would talk over everything while agents run, and the sweep only
//! hands over the changes notifications treat as news (a [`Kind`]), so working, idle and ended
//! never reach here.

use std::collections::VecDeque;
use std::time::Instant;

use super::{Kind, NOTIFY_BURST, NOTIFY_BURST_WINDOW};
use crate::elide::ELLIPSIS;
use crate::spoken::{terminals_failed, terminals_finished, terminals_need_you};

/// How many characters of an agent's question are read out. A permission prompt can quote a whole
/// command; past a sentence or so the listener has what they need to go and answer it.
const QUESTION_MAX: usize = 100;

/// How an announcement takes its turn (`NSAccessibilityPriorityLevel`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// Interrupts what VoiceOver is saying (needs you).
    High,
    /// Waits its turn (finished, failed).
    Medium,
}

/// One thing for VoiceOver to say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    pub text: String,
    pub priority: Priority,
}

/// One terminal's change in one sweep, as announcements see it.
#[derive(Debug, Clone, Copy)]
pub struct Said<'a> {
    /// The tab it happened in: a burst counts terminals, not changes.
    pub tab: i64,
    /// The terminal's name, as the sidebar shows it.
    pub name: &'a str,
    pub kind: Kind,
    /// The agent's question, for needs you; what went wrong, for a failure ("exit 101").
    pub question: Option<&'a str>,
}

/// Remembers what was announced lately, to coalesce bursts.
#[derive(Debug, Default)]
pub struct Announcer {
    recent: VecDeque<Told>,
}

#[derive(Debug)]
struct Told {
    at: Instant,
    tab: i64,
    heard: Heard,
}

/// What an announcement is about. Needs you, finished and failed are counted apart: a burst of
/// agents finishing doesn't turn the next question into a count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Heard {
    NeedsYou,
    Finished,
    Failed,
}

impl Heard {
    fn of(kind: Kind) -> Option<Heard> {
        match kind {
            // A program's own notification is the one it meant to send: heard like a question.
            Kind::NeedsYou | Kind::Notice => Some(Heard::NeedsYou),
            Kind::AgentDone | Kind::CommandDone => Some(Heard::Finished),
            Kind::AgentFailed | Kind::CommandFailed => Some(Heard::Failed),
            // A bell is not announced: it already sounded, and a terminal without an agent
            // can ring it far more often than anyone wants to hear about.
            Kind::Bell => None,
        }
    }

    fn priority(self) -> Priority {
        match self {
            Heard::NeedsYou => Priority::High,
            Heard::Finished | Heard::Failed => Priority::Medium,
        }
    }
}

impl Announcer {
    /// What VoiceOver says for the changes one sweep found (in the order given), at `at`: at most
    /// one announcement for needs you, then at most one each for finished and failed. A single
    /// terminal is named ("diff perf needs you: Approve cargo test?", "auth tokens finished",
    /// "build failed: exit 101"); several in this sweep,
    /// or from the [`NOTIFY_BURST`]th terminal within [`NOTIFY_BURST_WINDOW`] on, are said as the
    /// burst's count ("3 terminals need you"). A terminal heard again inside the window is still
    /// one terminal.
    pub fn announce(&mut self, at: Instant, changes: &[Said]) -> Vec<Announcement> {
        self.recent.retain(|told| at.saturating_duration_since(told.at) < NOTIFY_BURST_WINDOW);
        [Heard::NeedsYou, Heard::Finished, Heard::Failed]
            .into_iter()
            .filter_map(|heard| self.say(at, heard, changes))
            .collect()
    }

    fn say(&mut self, at: Instant, heard: Heard, changes: &[Said]) -> Option<Announcement> {
        let now: Vec<&Said> = changes.iter().filter(|said| Heard::of(said.kind) == Some(heard)).collect();
        let first = *now.first()?;
        self.recent.extend(now.iter().map(|said| Told { at, tab: said.tab, heard }));
        let alone = now.iter().all(|said| said.tab == first.tab);
        let burst = terminals(self.recent.iter().filter(|told| told.heard == heard).map(|told| told.tab));
        let text = match heard {
            _ if alone && burst < NOTIFY_BURST => named(first),
            Heard::NeedsYou => terminals_need_you(burst),
            Heard::Finished => terminals_finished(burst),
            Heard::Failed => terminals_failed(burst),
        };
        Some(Announcement { text, priority: heard.priority() })
    }
}

/// How many different terminals.
fn terminals(tabs: impl Iterator<Item = i64>) -> usize {
    let mut seen: Vec<i64> = Vec::new();
    for tab in tabs {
        if !seen.contains(&tab) {
            seen.push(tab);
        }
    }
    seen.len()
}

/// One terminal's announcement, by its name.
fn named(said: &Said) -> String {
    let name = if said.name.trim().is_empty() { "Terminal" } else { said.name.trim() };
    let words = said.question.map(shortened).filter(|q| !q.is_empty());
    match (Heard::of(said.kind), words) {
        (Some(Heard::NeedsYou), Some(words)) if said.kind == Kind::Notice => format!("{name}: {words}"),
        (Some(Heard::NeedsYou), None) if said.kind == Kind::Notice => format!("{name} sent a notification"),
        (Some(Heard::NeedsYou), Some(question)) => format!("{name} needs you: {question}"),
        (Some(Heard::NeedsYou), None) => format!("{name} needs you"),
        (Some(Heard::Failed), Some(words)) => format!("{name} failed: {words}"),
        (Some(Heard::Failed), None) => format!("{name} failed"),
        _ => format!("{name} finished"),
    }
}

/// The question on one line, cut after the last whole word that fits [`QUESTION_MAX`] (a single
/// word longer than that is cut inside), with an ellipsis for what was left out.
fn shortened(question: &str) -> String {
    let mut said = String::new();
    let mut length = 0;
    for word in question.split_whitespace() {
        let space = usize::from(!said.is_empty());
        let chars = word.chars().count();
        if length + space + chars > QUESTION_MAX {
            if said.is_empty() {
                said.extend(word.chars().take(QUESTION_MAX));
            }
            // "Run the tests…" rather than "Run the tests,…".
            let kept = said.trim_end_matches([',', ';', ':', '.', '-', '–', '—']).len();
            said.truncate(kept);
            said.push(ELLIPSIS);
            return said;
        }
        if space == 1 {
            said.push(' ');
        }
        said.push_str(word);
        length += space + chars;
    }
    said
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Announcement, Announcer, Priority, QUESTION_MAX, Said, shortened};
    use crate::notify::{Kind, NOTIFY_BURST_WINDOW};

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    fn needs<'a>(tab: i64, name: &'a str, question: Option<&'a str>) -> Said<'a> {
        Said { tab, name, kind: Kind::NeedsYou, question }
    }

    fn done(tab: i64, name: &str) -> Said<'_> {
        Said { tab, name, kind: Kind::AgentDone, question: None }
    }

    fn high(text: &str) -> Announcement {
        Announcement { text: text.to_string(), priority: Priority::High }
    }

    fn medium(text: &str) -> Announcement {
        Announcement { text: text.to_string(), priority: Priority::Medium }
    }

    #[test]
    fn the_three_sentences() {
        let t0 = Instant::now();
        let mut announcer = Announcer::default();
        assert_eq!(
            announcer.announce(t0, &[needs(1, "diff perf", Some("Approve cargo test?"))]),
            [high("diff perf needs you: Approve cargo test?")]
        );
        assert_eq!(
            announcer.announce(t0 + secs(1), &[done(2, "auth tokens")]),
            [medium("auth tokens finished")]
        );
        let three = [needs(3, "a", None), needs(4, "b", None), needs(5, "c", None)];
        assert_eq!(announcer.announce(t0 + secs(2), &three), [high("4 terminals need you")]);
        // The same, without the one before.
        assert_eq!(Announcer::default().announce(t0, &three), [high("3 terminals need you")]);
    }

    #[test]
    fn a_command_finishing_is_told_by_its_terminal() {
        let said = Said { tab: 1, name: "release", kind: Kind::CommandDone, question: Some("ignored") };
        assert_eq!(Announcer::default().announce(Instant::now(), &[said]), [medium("release finished")]);
    }

    #[test]
    fn needs_you_without_a_question_is_just_the_name() {
        let t0 = Instant::now();
        for question in [None, Some(""), Some("  \n ")] {
            let said = needs(1, "diff perf", question);
            assert_eq!(
                Announcer::default().announce(t0, &[said]),
                [high("diff perf needs you")],
                "{question:?}"
            );
        }
        let unnamed = needs(1, " ", Some(" Approve? "));
        assert_eq!(Announcer::default().announce(t0, &[unnamed]), [high("Terminal needs you: Approve?")]);
    }

    #[test]
    fn a_long_question_is_cut_at_a_word() {
        assert_eq!(shortened("Approve\n  cargo   test?"), "Approve cargo test?");
        let long = format!("Allow {} and more", "word ".repeat(30));
        let cut = shortened(&long);
        assert!(cut.ends_with("word…"), "{cut}");
        assert!(cut.chars().count() <= QUESTION_MAX + 1, "{cut}");
        // The punctuation where it was cut gives way to the ellipsis.
        let comma = format!("{}, then the rest", "x".repeat(QUESTION_MAX - 2));
        assert_eq!(shortened(&comma), format!("{}…", "x".repeat(QUESTION_MAX - 2)));
        // A word with no room for a boundary is cut inside.
        let path = "/".repeat(QUESTION_MAX * 2);
        assert_eq!(shortened(&path).chars().count(), QUESTION_MAX + 1);
        assert_eq!(shortened(&"x".repeat(QUESTION_MAX)), "x".repeat(QUESTION_MAX), "what fits stays whole");
    }

    #[test]
    fn several_in_one_sweep_are_one_count() {
        let t0 = Instant::now();
        let mut announcer = Announcer::default();
        assert_eq!(
            announcer.announce(t0, &[needs(1, "a", Some("Q?")), needs(2, "b", Some("R?"))]),
            [high("2 terminals need you")]
        );
        let finished = [done(3, "c"), Said { tab: 4, name: "d", kind: Kind::CommandDone, question: None }];
        assert_eq!(announcer.announce(t0 + secs(30), &finished), [medium("2 terminals finished")]);
    }

    #[test]
    fn a_burst_across_sweeps_is_said_as_its_count() {
        let t0 = Instant::now();
        let mut announcer = Announcer::default();
        assert_eq!(announcer.announce(t0, &[done(1, "auth tokens")]), [medium("auth tokens finished")]);
        assert_eq!(announcer.announce(t0 + secs(3), &[done(2, "tests")]), [medium("tests finished")]);
        assert_eq!(announcer.announce(t0 + secs(6), &[done(3, "release")]), [medium("3 terminals finished")]);
        assert_eq!(announcer.announce(t0 + secs(8), &[done(4, "docs")]), [medium("4 terminals finished")]);
        // Once the window has passed, a terminal is named again.
        let later = t0 + secs(8) + NOTIFY_BURST_WINDOW;
        assert_eq!(announcer.announce(later, &[done(5, "lint")]), [medium("lint finished")]);
    }

    #[test]
    fn spread_out_changes_stay_single() {
        let t0 = Instant::now();
        let mut announcer = Announcer::default();
        for (tab, at) in [0, 6, 12, 18, 24].into_iter().enumerate() {
            let said = needs(tab as i64, "prod", Some("Continue?"));
            assert_eq!(
                announcer.announce(t0 + secs(at), &[said]),
                [high("prod needs you: Continue?")],
                "at {at}s"
            );
        }
    }

    #[test]
    fn one_terminal_asking_again_is_still_named() {
        let t0 = Instant::now();
        let mut announcer = Announcer::default();
        for (n, question) in ["One?", "Two?", "Three?"].into_iter().enumerate() {
            assert_eq!(
                announcer.announce(t0 + secs(n as u64), &[needs(7, "diff perf", Some(question))]),
                [high(&format!("diff perf needs you: {question}"))]
            );
        }
    }

    #[test]
    fn needs_you_and_finished_are_counted_apart() {
        let t0 = Instant::now();
        let mut announcer = Announcer::default();
        let sweeps = [
            (needs(1, "a", None), "a needs you"),
            (done(2, "b"), "b finished"),
            (needs(3, "c", None), "c needs you"),
            (done(4, "d"), "d finished"),
        ];
        for (n, (said, text)) in sweeps.into_iter().enumerate() {
            let said_now = announcer.announce(t0 + secs(n as u64), &[said]);
            assert_eq!(said_now.len(), 1);
            assert_eq!(said_now[0].text, text, "two of each is no burst");
        }
        // The third of one kind is.
        assert_eq!(announcer.announce(t0 + secs(5), &[done(5, "e")]), [medium("3 terminals finished")]);
        assert_eq!(announcer.announce(t0 + secs(6), &[needs(6, "f", None)]), [high("3 terminals need you")]);
    }

    #[test]
    fn a_failure_is_named_with_what_went_wrong_and_counted_apart() {
        let t0 = Instant::now();
        let failed = |tab, name, question| Said { tab, name, kind: Kind::CommandFailed, question };
        let mut announcer = Announcer::default();
        assert_eq!(
            announcer.announce(t0, &[failed(1, "build", Some("exit 101"))]),
            [medium("build failed: exit 101")]
        );
        let agent = Said { tab: 2, name: "deploy", kind: Kind::AgentFailed, question: None };
        assert_eq!(announcer.announce(t0 + secs(1), &[agent]), [medium("deploy failed")]);
        // Failures and finishes don't make a burst of each other.
        assert_eq!(announcer.announce(t0 + secs(2), &[done(3, "tests")]), [medium("tests finished")]);
        assert_eq!(
            announcer.announce(t0 + secs(3), &[failed(4, "lint", None)]),
            [medium("3 terminals failed")]
        );
    }

    #[test]
    fn the_bell_says_nothing_and_counts_for_nothing() {
        let t0 = Instant::now();
        let mut announcer = Announcer::default();
        let bell = |tab| Said { tab, name: "prod", kind: Kind::Bell, question: None };
        assert_eq!(announcer.announce(t0, &[bell(1), bell(2), bell(3)]), []);
        assert_eq!(
            announcer.announce(t0 + secs(1), &[needs(4, "diff perf", None)]),
            [high("diff perf needs you")]
        );
        assert_eq!(announcer.announce(t0 + secs(2), &[]), []);
    }

    #[test]
    fn a_programs_notification_is_said_in_its_words() {
        let t0 = Instant::now();
        let mut announcer = Announcer::default();
        let notice = |question| Said { tab: 1, name: "deploy", kind: Kind::Notice, question };
        assert_eq!(announcer.announce(t0, &[notice(Some("v1.2 is live"))]), [high("deploy: v1.2 is live")]);
        let mut announcer = Announcer::default();
        assert_eq!(announcer.announce(t0, &[notice(None)]), [high("deploy sent a notification")]);
    }

    #[test]
    fn needs_you_comes_before_finished() {
        let t0 = Instant::now();
        let mut announcer = Announcer::default();
        let sweep = [done(1, "auth tokens"), needs(2, "diff perf", Some("Approve cargo test?"))];
        assert_eq!(
            announcer.announce(t0, &sweep),
            [high("diff perf needs you: Approve cargo test?"), medium("auth tokens finished")]
        );
    }
}
