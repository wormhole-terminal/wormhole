//! What changed in each tab since the app last looked (every tick, and right after an agent's
//! hook), turned into the events the notification rules act on. A tab is remembered by its id,
//! which is unique across windows, so one watch serves the whole app.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime};

use wormhole_core::agents::Foreground;
use wormhole_core::notify::{self, Change, Kind, State};
use wormhole_core::sidebar::command::ends_by_itself;

use crate::workspace::ambient::TabFacts;

/// A tab as it was last seen.
#[derive(Debug)]
struct Memory {
    state: State,
    since: Instant,
    agent: bool,
    foreground: Option<Foreground>,
    /// The number of the program's last notification (0: none).
    notice: u64,
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
            let now = Memory {
                state: facts.state,
                since: facts.since,
                agent: facts.agent.is_some(),
                foreground: facts.foreground.clone(),
                notice,
            };
            let before = self.memory.insert(facts.tab, now).unwrap_or(Memory {
                state: State::Idle,
                since: facts.since,
                agent: false,
                foreground: None,
                notice: 0,
            });
            // An agent that quit between two looks still made the change it was there for.
            let agent = before.agent || facts.agent.is_some();
            let worked =
                (before.state == State::Working).then(|| facts.since.saturating_duration_since(before.since));
            let noticed = notice != 0 && notice != before.notice;
            match notify::on_state(before.state, facts.state, agent) {
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
            if let Some(ended) = before.foreground
                && facts.foreground.as_ref().map(|f| f.started) != Some(ended.started)
            {
                let took = wall.duration_since(ended.started).unwrap_or_default();
                if !agent && ends_by_itself(&ended.command) {
                    self.finished.insert(facts.tab, (ended.command.clone(), took));
                }
                if notify::command_finished(&ended.command, took, command_after, agent) {
                    let command = Some(ended.command);
                    events.push(Event::Post {
                        tab: facts.tab,
                        kind: Kind::CommandDone,
                        took: Some(took),
                        command,
                    });
                }
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
    use wormhole_core::notify::{Kind, State};
    use wormhole_core::terminal::Notification;

    use super::{Event, Watch};
    use crate::terminal_view::ProgramNotice;
    use crate::workspace::ambient::TabFacts;

    fn tab(state: State, since: Instant) -> TabFacts {
        TabFacts {
            session: 1,
            tab: 7,
            name: "api".into(),
            state,
            since,
            agent: None,
            message: None,
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
        let events = watch.observe(&[facts], false, 10, SystemTime::now());
        assert_eq!(events, [Event::Post { tab: 7, kind: Kind::AgentDone, took: None, command: None }]);
    }
}
