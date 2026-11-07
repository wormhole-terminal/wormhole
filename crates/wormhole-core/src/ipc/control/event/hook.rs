//! A bus event as a Probe's hook sees it: the `probe::events` vocabulary read off what the bus tells.
//!
//! `probe` sits below `ipc` and holds the names as text (`tests/probes_events.rs` checks them against the
//! bus); this is the other direction, the one place that turns a bus [`Event`] into a [`HookEvent`] and its
//! fields, and says which topics a set of hook events needs. The event carries ids and numbers; the text it
//! holds (a command line, a title, a folder) is in the fields too, and what a Probe may be told of it is
//! the dispatcher's to decide (`probe::active`), never the bus's.

use std::collections::HashMap;

use super::{BlockPhase, Body, Event, Topic, Topics};
use crate::ipc::control::TerminalState;
use crate::probe::events::{self, EventData, Source, Value};
use crate::probe::manifest::HookEvent;
use crate::probe::rules;

fn state(state: TerminalState) -> Option<&'static str> {
    Some(match state {
        TerminalState::Idle => "idle",
        TerminalState::Working => "working",
        TerminalState::NeedsYou => "needs_you",
        TerminalState::Finished => "finished",
        TerminalState::Failed => "failed",
        TerminalState::Exited => "exited",
        TerminalState::Other => return None,
    })
}

/// The hook event `event` is, with its fields. `None` for what no hook listens to (splits, windows, output,
/// a terminal moving), for an event about no terminal, and for a state or phase this build has no word for.
pub fn hook_event(event: &Event) -> Option<(HookEvent, EventData)> {
    let terminal = event.terminal?;
    let mut fields = vec![("terminal".to_string(), Value::Number(terminal))];
    let mut text = |name: &str, value: &Option<String>| {
        if let Some(value) = value {
            fields.push((name.to_string(), Value::Text(value.clone())));
        }
    };
    let kind = match &event.body {
        Body::Status { status, was, agent, message } => {
            let (status, was) = (state(*status)?, state(*was)?);
            text("agent", agent);
            text("message", message);
            fields.push(("status".into(), Value::Text(status.into())));
            fields.push(("was".into(), Value::Text(was.into())));
            HookEvent::AgentStatus
        }
        Body::Block { block, phase, exit_code, took_ms, command, output_lines } => {
            let kind = match phase {
                BlockPhase::Started => HookEvent::CommandStarted,
                BlockPhase::Finished => HookEvent::CommandFinished,
                BlockPhase::Other => return None,
            };
            text("command", command);
            fields.push(("block".into(), Value::Number(*block)));
            if let Some(code) = exit_code {
                fields.push(("exit".into(), Value::Number(i64::from(*code))));
            }
            let mut number = |name: &str, value: &Option<u64>| {
                if let Some(value) = value {
                    fields.push((name.into(), Value::Number(i64::try_from(*value).unwrap_or(i64::MAX))));
                }
            };
            number("took_ms", took_ms);
            number("output_lines", output_lines);
            kind
        }
        Body::Git { branch, ahead, behind, dirty, untracked } => {
            text("branch", branch);
            for (name, value) in
                [("ahead", ahead), ("behind", behind), ("dirty", dirty), ("untracked", untracked)]
            {
                fields.push((name.into(), Value::Number(i64::from(*value))));
            }
            HookEvent::GitChanged
        }
        Body::Folder { folder } => {
            text("folder", folder);
            HookEvent::FolderChanged
        }
        Body::Bell => HookEvent::Bell,
        Body::Title { title, name } => {
            text("title", title);
            fields.push(("name".into(), Value::Text(name.clone())));
            HookEvent::Title
        }
        Body::TerminalOpened { folder } => {
            text("folder", folder);
            HookEvent::TerminalOpened
        }
        Body::TerminalClosed => HookEvent::TerminalClosed,
        _ => return None,
    };
    Some((kind, EventData::new(fields)))
}

/// The topics a subscriber needs to hear every one of `events`.
pub fn topics(events: impl IntoIterator<Item = HookEvent>) -> Topics {
    events
        .into_iter()
        .filter_map(|event| match events::spec(event).source {
            Source::Bus { topic, .. } => Topic::from_name(topic),
        })
        .fold(Topics::default(), Topics::with)
}

/// Terminals whose printed lines are numbered at once. A feed with more forgets them all: the numbers
/// start again, which only a hook that cares about `line_no` could tell.
const NUMBERED_TERMINALS: usize = 256;

/// Counts the lines each terminal printed, so an `output.match` can say which line it is.
#[derive(Debug, Default)]
pub struct Printed {
    seen: HashMap<i64, i64>,
}

impl Printed {
    /// One event per finished line of what `event` says a terminal printed, each cut to what a pattern
    /// scans. `line_no` counts lines since the feed began (lines the bus left out count too). A full-screen
    /// program's screen has no lines, and the cursor's own, unfinished line is not told.
    pub fn lines(&mut self, event: &Event) -> Vec<EventData> {
        let (Some(terminal), Body::Output { lines, skipped, .. }) = (event.terminal, &event.body) else {
            return Vec::new();
        };
        if self.seen.len() >= NUMBERED_TERMINALS && !self.seen.contains_key(&terminal) {
            self.seen.clear();
        }
        let seen = self.seen.entry(terminal).or_insert(0);
        *seen = seen.saturating_add(i64::try_from(*skipped).unwrap_or(i64::MAX));
        let mut told = Vec::with_capacity(lines.len());
        for line in lines {
            *seen = seen.saturating_add(1);
            told.push(EventData::new([
                ("terminal".to_string(), Value::Number(terminal)),
                ("line_no".to_string(), Value::Number(*seen)),
                ("line".to_string(), Value::Text(rules::scanned(line).to_string())),
            ]));
        }
        told
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(terminal: Option<i64>, body: Body) -> Event {
        Event { window: Some(1), terminal, body }
    }

    #[test]
    fn a_finished_command_is_a_hook_event_with_its_numbers() {
        let body = Body::Block {
            block: 7,
            phase: BlockPhase::Finished,
            exit_code: Some(2),
            took_ms: Some(61_000),
            command: Some("make".into()),
            output_lines: Some(40),
        };
        let (kind, data) = hook_event(&event(Some(3), body)).expect("a hook event");
        assert_eq!(kind, HookEvent::CommandFinished);
        assert_eq!((data.number("terminal"), data.number("block")), (Some(3), Some(7)));
        assert_eq!((data.number("exit"), data.number("took_ms")), (Some(2), Some(61_000)));
        assert_eq!(data.number("output_lines"), Some(40));
        assert_eq!(data.text("command"), Some("make"));
        assert!(data.problems(events::spec(kind)).is_empty(), "{:?}", data.problems(events::spec(kind)));
    }

    #[test]
    fn what_no_hook_listens_to_is_none() {
        assert!(hook_event(&event(Some(1), Body::WindowOpened)).is_none());
        assert!(hook_event(&event(None, Body::Bell)).is_none(), "about no terminal");
        let status = Body::Status {
            status: TerminalState::Other,
            was: TerminalState::Idle,
            agent: None,
            message: None,
        };
        assert!(hook_event(&event(Some(1), status)).is_none(), "a state this build has no word for");
    }

    #[test]
    fn topics_are_the_ones_the_events_come_from() {
        let wanted = topics([
            HookEvent::CommandFinished,
            HookEvent::Bell,
            HookEvent::GitChanged,
            HookEvent::OutputMatch,
        ]);
        assert_eq!(wanted.names(), ["bell", "blocks", "git", "output"]);
        assert!(topics([]).is_empty());
    }

    #[test]
    fn printed_lines_are_numbered_and_cut_to_what_a_pattern_scans() {
        let long = "x".repeat(rules::MAX_SCAN_BYTES + 100);
        let output = |lines: &[&str], skipped| {
            let lines = lines.iter().map(|l| l.to_string()).collect();
            event(Some(4), Body::Output { screen: "main".into(), lines, partial: None, skipped, lost: false })
        };
        let mut printed = Printed::default();
        let told = printed.lines(&output(&["a", &long], 0));
        let numbers: Vec<_> = told.iter().map(|d| d.number("line_no")).collect();
        assert_eq!(numbers, [Some(1), Some(2)]);
        assert_eq!(told[1].text("line").map(str::len), Some(rules::MAX_SCAN_BYTES));
        assert_eq!(told[0].number("terminal"), Some(4));
        let next = printed.lines(&output(&["b"], 3));
        assert_eq!(next[0].number("line_no"), Some(6), "three left out still count");
        assert!(printed.lines(&event(Some(4), Body::Bell)).is_empty());
    }

    #[test]
    fn a_git_change_is_numbers_and_a_branch_name() {
        let body = Body::Git { branch: Some("main".into()), ahead: 1, behind: 0, dirty: 3, untracked: 2 };
        let (kind, data) = hook_event(&event(Some(3), body)).expect("a hook event");
        assert_eq!(kind, HookEvent::GitChanged);
        assert_eq!(
            (data.number("ahead"), data.number("dirty"), data.number("untracked")),
            (Some(1), Some(3), Some(2))
        );
        assert_eq!(data.text("branch"), Some("main"));
        assert!(data.problems(events::spec(kind)).is_empty());
    }
}
