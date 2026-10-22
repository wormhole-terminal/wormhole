//! What changed in each tab since the app last looked (every tick, and right after an agent's
//! hook), turned into the events the notification rules act on. A tab is remembered by its id,
//! which is unique across windows, so one watch serves the whole app.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

use wormhole_core::agents::Foreground;
use wormhole_core::notify::{self, Change, Kind, Source, State};
use wormhole_core::view::sidebar::command::ends_by_itself;

use crate::workspace::ambient::TabFacts;

/// A tab as it was last seen.
#[derive(Debug)]
struct Memory {
    state: State,
    since: Instant,
    source: Source,
    foreground: Option<Foreground>,
    /// The number of the program's last notification (0: none).
    notice: u64,
    /// The number of the last command failure told about (0: none).
    failure: u64,
}

/// Something worth acting on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Event {
    /// `kind` happened in `tab`. `took`: how long the agent worked or the command ran;
    /// `command`: the command that ended.
    Post { tab: i64, kind: Kind, took: Option<Duration>, command: Option<String> },
    /// The tab moved on: its notice says nothing true any more.
    Retract(i64),
    /// The user looks at the tab: what is up about it goes.
    Seen(i64),
}

#[derive(Debug, Default)]
pub(super) struct Watch {
    memory: HashMap<i64, Memory>,
    /// The last command that ended by itself in each tab, and how long it ran (a finished
    /// terminal's row says it).
    pub finished: HashMap<i64, (String, Duration)>,
}

impl Watch {
    /// Compare `tabs` with what was seen before. A tab seen for the first time counts as having
    /// been idle (every terminal starts so; a hook may arrive before the first look). Tabs that
    /// are gone are forgotten. `command_after`: `notify_command_after`.
    pub(super) fn observe(
        &mut self,
        tabs: &[TabFacts],
        app_active: bool,
        command_after: u32,
        wall: SystemTime,
    ) -> Vec<Event> {
        let mut events = Vec::new();
        for facts in tabs {
            let notice = facts.notice.as_ref().map_or(0, |n| n.serial);
            let failure = facts.failure.as_ref().map_or(0, |f| f.serial);
            let now = Memory {
                state: facts.state,
                since: facts.since,
                source: facts.source,
                foreground: facts.foreground.clone(),
                notice,
                failure,
            };
            let before = self.memory.insert(facts.tab, now).unwrap_or(Memory {
                state: State::Idle,
                since: facts.since,
                source: Source::Plain,
                foreground: None,
                notice: 0,
                failure: 0,
            });
            // An agent or program that quit between two looks still made the change it was there
            // for.
            let source = if facts.source.told() { facts.source } else { before.source };
            let worked =
                (before.state == State::Working).then(|| facts.since.saturating_duration_since(before.since));
            let noticed = notice != 0 && notice != before.notice;
            // A failing exit code the shell gave since the last look.
            let failed = facts.failure.as_ref().filter(|f| f.serial != before.failure);
            match notify::on_state(before.state, facts.state, source) {
                // The program's notification asked for you with its own words: it is told
                // instead of the bell or question its arrival counted as.
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
                // The shell's exit code, when it gave one since the command began, says how it
                // ended: a command that failed did not "finish", whichever look told about it.
                let its_failure = facts.failure.as_ref().filter(|f| f.at.elapsed() <= ran);
                match its_failure {
                    Some(failure) => {
                        failure_told = true;
                        let took = failure.took.unwrap_or(ran);
                        if failed.is_some()
                            && notify::command_failed(&ended.command, took, command_after, told)
                        {
                            events.push(Event::Post {
                                tab: facts.tab,
                                kind: Kind::CommandFailed,
                                took: Some(took),
                                command: Some(ended.command),
                            });
                        }
                    }
                    None if notify::command_finished(&ended.command, ran, command_after, told) => {
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
            // The shell's word without a command seen leaving the foreground (the mark came first,
            // or the command was gone before anyone looked): the command it names is the one.
            if let (Some(failure), false) = (failed, failure_told)
                && let (Some(command), Some(took)) = (failure.command.as_deref(), failure.took)
                && notify::command_failed(command, took, command_after, source.told())
            {
                events.push(Event::Post {
                    tab: facts.tab,
                    kind: Kind::CommandFailed,
                    took: Some(took),
                    command: Some(command.to_string()),
                });
            }
            if facts.looking(app_active) {
                events.push(Event::Seen(facts.tab));
            }
        }
        self.memory.retain(|tab, _| tabs.iter().any(|facts| facts.tab == *tab));
        self.finished.retain(|tab, _| tabs.iter().any(|facts| facts.tab == *tab));
        events
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant, SystemTime};

    use wormhole_core::agents::{AgentId, Foreground};
    use wormhole_core::notify::{Kind, Source, State};
    use wormhole_core::terminal::Notification;

    use super::{Event, Watch};
    use crate::terminal_view::{Failure, ProgramNotice};
    use crate::workspace::ambient::TabFacts;

    fn tab(state: State, since: Instant) -> TabFacts {
        TabFacts {
            session: 1,
            tab: 7,
            name: "api".into(),
            state,
            since,
            agent: None,
            source: Source::Plain,
            program: None,
            message: None,
            reported: false,
            failure: None,
            notice: None,
            foreground: None,
            folder: None,
            branch: None,
            changed: None,
            selected: false,
            key: false,
        }
    }

    #[test]
    fn a_programs_notification_is_told_in_place_of_the_bell_it_counted_as() {
        let notice = |serial, body: &str| {
            Some(ProgramNotice {
                serial,
                notification: Notification { title: String::new(), body: body.into() },
            })
        };
        let mut watch = Watch::default();
        let mut facts = tab(State::Idle, Instant::now());
        watch.observe(&[facts.clone()], false, 10, SystemTime::now());
        // The notification counted as the bell: the tab needs you, and one event says so.
        facts.state = State::NeedsYou;
        facts.notice = notice(1, "Build finished");
        let events = watch.observe(&[facts.clone()], false, 10, SystemTime::now());
        assert_eq!(events, [Event::Post { tab: 7, kind: Kind::Notice, took: None, command: None }]);
        // The same notice again changes nothing; a new one is told even without a state change.
        assert!(watch.observe(&[facts.clone()], false, 10, SystemTime::now()).is_empty());
        facts.notice = notice(2, "Tests passed");
        let events = watch.observe(&[facts.clone()], false, 10, SystemTime::now());
        assert_eq!(events, [Event::Post { tab: 7, kind: Kind::Notice, took: None, command: None }]);
        // Moving on retracts as for any other notice.
        facts.state = State::Working;
        assert_eq!(watch.observe(&[facts], false, 10, SystemTime::now()), [Event::Retract(7)]);
    }

    #[test]
    fn a_tab_seen_for_the_first_time_counts_from_idle() {
        let mut watch = Watch::default();
        assert!(watch.observe(&[tab(State::Idle, Instant::now())], false, 10, SystemTime::now()).is_empty());
        let mut other = tab(State::NeedsYou, Instant::now());
        other.tab = 8;
        let events = watch.observe(&[other.clone()], false, 10, SystemTime::now());
        assert_eq!(events, [Event::Post { tab: 8, kind: Kind::Bell, took: None, command: None }]);
        assert!(watch.observe(&[other], false, 10, SystemTime::now()).is_empty(), "no change, no event");
    }

    #[test]
    fn an_agent_that_finishes_says_how_long_it_worked() {
        let t0 = Instant::now();
        let mut watch = Watch::default();
        let mut facts = tab(State::Working, t0);
        facts.agent = Some(AgentId::Claude);
        facts.source = Source::Agent;
        watch.observe(&[facts.clone()], false, 10, SystemTime::now());
        facts.state = State::Done;
        facts.since = t0 + Duration::from_secs(252);
        let events = watch.observe(&[facts.clone()], false, 10, SystemTime::now());
        assert_eq!(
            events,
            [Event::Post {
                tab: 7,
                kind: Kind::AgentDone,
                took: Some(Duration::from_secs(252)),
                command: None
            }]
        );
        facts.state = State::Working;
        assert_eq!(watch.observe(&[facts], false, 10, SystemTime::now()), [Event::Retract(7)]);
    }

    #[test]
    fn a_plain_command_that_ends_is_remembered_and_told_past_the_threshold() {
        let wall = SystemTime::now();
        let t0 = Instant::now();
        let mut watch = Watch::default();
        let mut facts = tab(State::Working, t0);
        facts.foreground =
            Some(Foreground { command: "cargo build".into(), started: wall - Duration::from_secs(38) });
        watch.observe(&[facts.clone()], false, 10, wall);
        facts.foreground = None;
        facts.state = State::Idle;
        let events = watch.observe(&[facts.clone()], false, 10, wall);
        assert!(events.contains(&Event::Post {
            tab: 7,
            kind: Kind::CommandDone,
            took: Some(Duration::from_secs(38)),
            command: Some("cargo build".into()),
        }));
        assert_eq!(watch.finished.get(&7), Some(&("cargo build".to_string(), Duration::from_secs(38))));

        // A short one is remembered for its row, not told.
        facts.foreground = Some(Foreground { command: "ls".into(), started: wall - Duration::from_secs(1) });
        watch.observe(&[facts.clone()], false, 10, wall);
        facts.foreground = None;
        let events = watch.observe(&[facts.clone()], false, 10, wall);
        assert!(!events.iter().any(|e| matches!(e, Event::Post { kind: Kind::CommandDone, .. })));
        assert_eq!(watch.finished.get(&7).map(|(c, _)| c.as_str()), Some("ls"));

        // One command right after another between two looks: the first one ended.
        facts.foreground =
            Some(Foreground { command: "make".into(), started: wall - Duration::from_secs(60) });
        watch.observe(&[facts.clone()], false, 10, wall);
        facts.foreground = Some(Foreground { command: "make test".into(), started: wall });
        let events = watch.observe(&[facts.clone()], false, 10, wall);
        assert!(events.iter().any(|e| matches!(e, Event::Post { kind: Kind::CommandDone, .. })));
    }

    fn failure(serial: u64, code: i32, took: Option<Duration>) -> Failure {
        Failure {
            serial,
            code,
            at: Instant::now(),
            command: Some("cargo build".into()),
            took,
            last_line: Some("error[E0425]".into()),
        }
    }

    /// The command leaves the foreground at the look the shell's exit code arrives at: one
    /// "failed", and no "finished".
    #[test]
    fn a_command_that_fails_is_told_as_failed_and_not_as_finished() {
        let wall = SystemTime::now();
        let mut watch = Watch::default();
        let mut facts = tab(State::Working, Instant::now());
        facts.foreground =
            Some(Foreground { command: "cargo build".into(), started: wall - Duration::from_secs(38) });
        watch.observe(&[facts.clone()], false, 10, wall);
        facts.foreground = None;
        facts.state = State::Failed;
        facts.failure = Some(failure(1, 101, Some(Duration::from_secs(38))));
        let events = watch.observe(&[facts.clone()], false, 10, wall);
        assert_eq!(
            events,
            [Event::Post {
                tab: 7,
                kind: Kind::CommandFailed,
                took: Some(Duration::from_secs(38)),
                command: Some("cargo build".into()),
            }]
        );
        // The same failure is not told twice; opening the tab takes the notice down.
        assert!(watch.observe(&[facts.clone()], false, 10, wall).is_empty());
        facts.state = State::Idle;
        assert_eq!(watch.observe(&[facts], false, 10, wall), [Event::Retract(7)]);
    }

    /// The shell's word comes a look before the command is seen leaving the foreground (the marks
    /// are exact, the process list is polled): told then, and not again as "finished" when the
    /// foreground catches up.
    #[test]
    fn a_failure_told_before_the_foreground_catches_up_is_not_finished_afterwards() {
        let wall = SystemTime::now();
        let mut watch = Watch::default();
        let mut facts = tab(State::Working, Instant::now());
        facts.foreground =
            Some(Foreground { command: "cargo build".into(), started: wall - Duration::from_secs(38) });
        watch.observe(&[facts.clone()], false, 10, wall);
        facts.state = State::Failed;
        facts.failure = Some(failure(1, 101, Some(Duration::from_secs(38))));
        let events = watch.observe(&[facts.clone()], false, 10, wall);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], Event::Post { kind: Kind::CommandFailed, .. }), "{events:?}");
        facts.foreground = None;
        let events = watch.observe(&[facts], false, 10, wall);
        assert!(events.is_empty(), "already told: {events:?}");
    }

    #[test]
    fn a_quick_failure_changes_the_row_and_tells_nothing() {
        let wall = SystemTime::now();
        let mut watch = Watch::default();
        let mut facts = tab(State::Idle, Instant::now());
        watch.observe(&[facts.clone()], false, 10, wall);
        facts.state = State::Failed;
        facts.failure = Some(failure(1, 1, Some(Duration::from_secs(2))));
        assert!(watch.observe(&[facts.clone()], false, 10, wall).is_empty(), "past the threshold only");
        // Another one, long this time, is.
        facts.failure = Some(failure(2, 2, Some(Duration::from_secs(40))));
        let events = watch.observe(&[facts], false, 10, wall);
        assert!(matches!(events[..], [Event::Post { kind: Kind::CommandFailed, .. }]), "{events:?}");
    }

    /// A program that reports itself (no known agent) is told like an agent: it asks, finishes
    /// and fails by what it says, never by the bell or the command-finished rule.
    #[test]
    fn a_reporting_program_asks_finishes_and_fails_like_an_agent() {
        let wall = SystemTime::now();
        let t0 = Instant::now();
        let mut watch = Watch::default();
        let mut facts = tab(State::Working, t0);
        facts.source = Source::Program;
        facts.program = Some("brew".into());
        facts.foreground =
            Some(Foreground { command: "brew upgrade".into(), started: wall - Duration::from_secs(60) });
        watch.observe(&[facts.clone()], false, 10, wall);
        facts.state = State::NeedsYou;
        facts.since = t0 + Duration::from_secs(5);
        let events = watch.observe(&[facts.clone()], false, 10, wall);
        let five = Some(Duration::from_secs(5));
        assert_eq!(events, [Event::Post { tab: 7, kind: Kind::NeedsYou, took: five, command: None }]);
        facts.state = State::Working;
        assert_eq!(watch.observe(&[facts.clone()], false, 10, wall), [Event::Retract(7)]);
        facts.state = State::Done;
        facts.since = t0 + Duration::from_secs(65);
        let events = watch.observe(&[facts.clone()], false, 10, wall);
        assert_eq!(
            events,
            [Event::Post {
                tab: 7,
                kind: Kind::AgentDone,
                took: Some(Duration::from_secs(60)),
                command: None
            }]
        );
        facts.state = State::Working;
        watch.observe(&[facts.clone()], false, 10, wall);
        facts.state = State::Failed;
        let events = watch.observe(&[facts.clone()], false, 10, wall);
        assert!(matches!(events[..], [Event::Post { kind: Kind::AgentFailed, .. }]), "{events:?}");
        // The command ending is the program's own business: no "finished" for brew itself.
        facts.foreground = None;
        let events = watch.observe(&[facts], false, 10, wall);
        assert!(events.is_empty(), "{events:?}");
    }

    #[test]
    fn looking_at_a_tab_takes_its_notices_down_and_closed_tabs_are_forgotten() {
        let mut watch = Watch::default();
        let mut facts = tab(State::Idle, Instant::now());
        watch.observe(&[facts.clone()], true, 10, SystemTime::now());
        facts.selected = true;
        facts.key = true;
        assert_eq!(watch.observe(&[facts.clone()], true, 10, SystemTime::now()), [Event::Seen(7)]);
        assert!(
            watch.observe(&[facts.clone()], false, 10, SystemTime::now()).is_empty(),
            "Wormhole is behind"
        );
        // Closed: forgotten, so a tab seen again starts from idle.
        watch.observe(&[], true, 10, SystemTime::now());
        facts.state = State::Done;
        facts.agent = Some(AgentId::Claude);
        facts.source = Source::Agent;
        let events = watch.observe(&[facts], false, 10, SystemTime::now());
        assert_eq!(events, [Event::Post { tab: 7, kind: Kind::AgentDone, took: None, command: None }]);
    }
}
