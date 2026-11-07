//! A bus event as a Probe's hook sees it: the `probe::events` vocabulary read off what the bus tells.
//!
//! `probe` sits below `ipc` and holds the names as text (`tests/probes_events.rs` checks them against the
//! bus); this is the other direction, the one place that turns a bus [`Event`] into a [`HookEvent`] and its
//! fields, and says which topics a set of hook events needs. The event carries ids and numbers; the text it
//! holds (a command line, a title, a folder) is in the fields too, and what a Probe may be told of it is
//! the dispatcher's to decide (`probe::active`), never the bus's.

use super::{BlockPhase, Body, Event, Topic, Topics};
use crate::ipc::control::TerminalState;
use crate::probe::events::{self, EventData, Source, Value};
use crate::probe::manifest::HookEvent;

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
        Body::Block { block, phase, exit_code, took_ms, command } => {
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
            if let Some(took) = took_ms {
                fields.push(("took_ms".into(), Value::Number(i64::try_from(*took).unwrap_or(i64::MAX))));
            }
            kind
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

/// The topics a subscriber needs to hear every one of `events`: those the bus tells. An event with no topic
/// of its own (`git.changed`) and `output.match` (which needs what terminals print, not delivered by this
/// feed) add none.
pub fn topics(events: impl IntoIterator<Item = HookEvent>) -> Topics {
    events
        .into_iter()
        .filter(|event| *event != HookEvent::OutputMatch)
        .filter_map(|event| match events::spec(event).source {
            Source::Bus { topic, .. } => Topic::from_name(topic),
            Source::GitWatch => None,
        })
        .fold(Topics::default(), Topics::with)
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
        };
        let (kind, data) = hook_event(&event(Some(3), body)).expect("a hook event");
        assert_eq!(kind, HookEvent::CommandFinished);
        assert_eq!((data.number("terminal"), data.number("block")), (Some(3), Some(7)));
        assert_eq!((data.number("exit"), data.number("took_ms")), (Some(2), Some(61_000)));
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
        assert_eq!(wanted.names(), ["bell", "blocks"]);
        assert!(topics([]).is_empty());
    }
}
