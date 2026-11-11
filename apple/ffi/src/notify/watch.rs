//! What changed in each terminal since the last look, turned into the events the rules act on. The rules are
//! core's (`notify::on_state`, `command_finished`, `command_failed`); this is the memory that turns two looks
//! into a change, as the app's `notifications/watch.rs` keeps it, over plain facts and a clock the caller
//! passes in.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

use wormhole_core::notify::{self, Change, Kind, Source, State};
use wormhole_core::view::sidebar::command::ends_by_itself;

/// A command in the foreground and when it started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Running {
    pub command: String,
    pub started: SystemTime,
}

#[derive(Debug, Clone)]
pub(super) struct Failure {
    pub serial: u64,
    pub at: Instant,
    pub command: Option<String>,
    pub took: Option<Duration>,
}

/// One terminal as read on this look, with core's enums and instants.
#[derive(Debug, Clone)]
pub(super) struct Tab {
    pub tab: i64,
    pub state: State,
    pub since: Instant,
    pub source: Source,
    pub notice_serial: u64,
    pub failure: Option<Failure>,
    pub foreground: Option<Running>,
    pub looking: bool,
    pub command_after: u32,
}

/// Something worth acting on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Event {
    /// `kind` happened in `tab`; `took`: how long the agent worked or the command ran; `command`: the one that ended.
    Post { tab: i64, kind: Kind, took: Option<Duration>, command: Option<String> },
    /// The terminal moved on: its notice says nothing true any more.
    Retract(i64),
    /// The person looks at it: what is up about it goes.
    Seen(i64),
}

/// A tab as it was last seen.
#[derive(Debug)]
struct Memory {
    state: State,
    since: Instant,
    source: Source,
    foreground: Option<Running>,
    notice: u64,
    failure: u64,
}

#[derive(Debug, Default)]
pub(super) struct Watch {
    memory: HashMap<i64, Memory>,
    /// The last command that ended by itself in each tab, and how long it ran.
    pub finished: HashMap<i64, (String, Duration)>,
}

impl Watch {
    /// Compare `tabs` with what was seen before. A tab seen for the first time counts as having been idle (every
    /// terminal starts so; a hook may arrive before the first look); tabs that are gone are forgotten. `now` is
    /// the monotonic clock of this look and `wall` the wall clock.
    pub(super) fn observe(&mut self, tabs: &[Tab], now: Instant, wall: SystemTime) -> Vec<Event> {
        let mut events = Vec::new();
        for facts in tabs {
            let failure = facts.failure.as_ref().map_or(0, |f| f.serial);
            let current = Memory {
                state: facts.state,
                since: facts.since,
                source: facts.source,
                foreground: facts.foreground.clone(),
                notice: facts.notice_serial,
                failure,
            };
            let before = self.memory.insert(facts.tab, current).unwrap_or(Memory {
                state: State::Idle,
                since: facts.since,
                source: Source::Plain,
                foreground: None,
                notice: 0,
                failure: 0,
            });
            // An agent or program that quit between two looks still made the change it was there for.
            let source = if facts.source.told() { facts.source } else { before.source };
            let worked =
                (before.state == State::Working).then(|| facts.since.saturating_duration_since(before.since));
            let noticed = facts.notice_serial != 0 && facts.notice_serial != before.notice;
            let failed = facts.failure.as_ref().filter(|f| f.serial != before.failure);
            match notify::on_state(before.state, facts.state, source) {
                // The program's notification asked for you with its own words: it is told instead of the
                // bell or question its arrival counted as.
                Change::Post(Kind::Bell | Kind::NeedsYou) if noticed => {}
                Change::Post(kind) => {
                    events.push(Event::Post { tab: facts.tab, kind, took: worked, command: None })
                }
                Change::Retract => events.push(Event::Retract(facts.tab)),
                Change::Keep => {}
            }
            if noticed {
                events.push(Event::Post { tab: facts.tab, kind: Kind::Notice, took: None, command: None });
            }
            let mut failure_told = false;
            if let Some(ended) = before.foreground
                && facts.foreground.as_ref().map(|f| f.started) != Some(ended.started)
            {
                let ran = wall.duration_since(ended.started).unwrap_or_default();
                let told = source.told();
                if !told && ends_by_itself(&ended.command) {
                    self.finished.insert(facts.tab, (ended.command.clone(), ran));
                }
                // The shell's exit code, when it gave one since the command began, says how it ended: a command
                // that failed did not "finish", whichever look told about it.
                let its_failure =
                    facts.failure.as_ref().filter(|f| now.saturating_duration_since(f.at) <= ran);
                match its_failure {
                    Some(failure) => {
                        failure_told = true;
                        let took = failure.took.unwrap_or(ran);
                        if failed.is_some()
                            && notify::command_failed(&ended.command, took, facts.command_after, told)
                        {
                            events.push(Event::Post {
                                tab: facts.tab,
                                kind: Kind::CommandFailed,
                                took: Some(took),
                                command: Some(ended.command),
                            });
                        }
                    }
                    None if notify::command_finished(&ended.command, ran, facts.command_after, told) => {
                        events.push(Event::Post {
                            tab: facts.tab,
                            kind: Kind::CommandDone,
                            took: Some(ran),
                            command: Some(ended.command),
                        });
                    }
                    None => {}
                }
            }
            // The shell's word without a command seen leaving the foreground (the mark came first, or the
            // command was gone before anyone looked): the command it names is the one.
            if let (Some(failure), false) = (failed, failure_told)
                && let (Some(command), Some(took)) = (failure.command.as_deref(), failure.took)
                && notify::command_failed(command, took, facts.command_after, source.told())
            {
                events.push(Event::Post {
                    tab: facts.tab,
                    kind: Kind::CommandFailed,
                    took: Some(took),
                    command: Some(command.to_string()),
                });
            }
            if facts.looking {
                events.push(Event::Seen(facts.tab));
            }
        }
        self.memory.retain(|tab, _| tabs.iter().any(|facts| facts.tab == *tab));
        self.finished.retain(|tab, _| tabs.iter().any(|facts| facts.tab == *tab));
        events
    }
}
