//! What travels over the socket: one JSON object per line.

use serde::{Deserialize, Serialize};

use crate::agents::AgentId;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Message {
    Hook {
        /// The tab the hook runs in (`WORMHOLE_TAB_ID`).
        tab: String,
        /// Hooks installed before other agents were supported are Claude's.
        #[serde(default = "claude")]
        agent: AgentId,
        event: String,
        #[serde(default)]
        message: Option<String>,
    },
    /// A second launch asks the running instance to come to the front.
    Activate,
    /// An agent's plan usage limits, from its status line (`Wormhole statusline`).
    Usage { usage: crate::agents::usage::Usage },
    /// What an agent's status line says about one terminal, the model and its context window
    /// (`Wormhole statusline`, from inside the terminal: `tab` is `WORMHOLE_TAB_ID`). A new
    /// message: an older app that doesn't know it skips the line, and keeps the usage that
    /// travels beside it.
    Gauge { tab: String, gauge: crate::agents::usage::Gauge },
    /// A control request (`Wormhole list`, `send-text`, …), answered on the same connection.
    Control(super::control::Request),
}

fn claude() -> AgentId {
    AgentId::Claude
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::Message;
    use crate::agents::AgentId;

    /// Compiles only while every variant is listed: a new one must get a line in [`golden`] too.
    fn kind(message: &Message) -> &'static str {
        match message {
            Message::Hook { .. } => "hook",
            Message::Activate => "activate",
            Message::Usage { .. } => "usage",
            Message::Gauge { .. } => "gauge",
            Message::Control(_) => "control",
        }
    }

    fn hook(tab: &str, agent: AgentId, event: &str, message: Option<&str>) -> Message {
        Message::Hook { tab: tab.into(), agent, event: event.into(), message: message.map(str::to_string) }
    }

    /// Every variant as it goes over the wire. The installed agent hooks run an older `Wormhole`
    /// binary against a newer app (and the reverse) for as long as an agent session stays open, so
    /// a change here is a compatibility change: update the lines on purpose, never to make a
    /// failing test pass.
    fn golden() -> Vec<(Message, &'static str)> {
        vec![
            (
                hook("7", AgentId::Claude, "Stop", None),
                r#"{"kind":"hook","tab":"7","agent":"claude","event":"Stop","message":null}"#,
            ),
            (
                hook("12", AgentId::Codex, "Notification:permission_prompt", Some("needs input")),
                r#"{"kind":"hook","tab":"12","agent":"codex","event":"Notification:permission_prompt","message":"needs input"}"#,
            ),
            (
                hook("3", AgentId::OpenCode, "session.idle", None),
                r#"{"kind":"hook","tab":"3","agent":"opencode","event":"session.idle","message":null}"#,
            ),
            (Message::Activate, r#"{"kind":"activate"}"#),
            (
                Message::Usage {
                    usage: crate::agents::usage::Usage {
                        agent: AgentId::Claude,
                        five_hour: Some(crate::agents::usage::Window {
                            used: 25.5,
                            resets_at: 1_800_002_040,
                        }),
                        seven_day: None,
                        limits: Vec::new(),
                        response_tokens: None,
                        updated: 1_800_000_000,
                    },
                },
                r#"{"kind":"usage","usage":{"agent":"claude","five_hour":{"used":25.5,"resets_at":1800002040},"seven_day":null,"updated":1800000000}}"#,
            ),
            (
                Message::Gauge {
                    tab: "7".into(),
                    gauge: crate::agents::usage::Gauge {
                        agent: AgentId::Claude,
                        model: Some("Opus 4.5".into()),
                        context: Some(41.5),
                    },
                },
                r#"{"kind":"gauge","tab":"7","gauge":{"agent":"claude","model":"Opus 4.5","context":41.5}}"#,
            ),
            (
                Message::Control(crate::ipc::control::Request::new(
                    crate::ipc::control::Action::SendText {
                        target: crate::ipc::control::Target::Tab(4),
                        text: "hi".into(),
                        submit: false,
                    },
                    Some("7".into()),
                )),
                r#"{"kind":"control","version":1,"from":"7","action":"send_text","target":{"kind":"tab","id":4},"text":"hi"}"#,
            ),
        ]
    }

    #[test]
    fn every_variant_has_a_golden_line() {
        let covered: BTreeSet<_> = golden().iter().map(|(message, _)| kind(message)).collect();
        assert_eq!(covered, BTreeSet::from(["activate", "control", "gauge", "hook", "usage"]));
    }

    #[test]
    fn messages_serialize_to_the_golden_lines() {
        for (message, line) in golden() {
            assert_eq!(serde_json::to_string(&message).expect("serializes"), line);
        }
    }

    #[test]
    fn the_golden_lines_parse_back() {
        for (message, line) in golden() {
            let parsed: Message = serde_json::from_str(line).expect("parses");
            // `Message` has no `PartialEq`; the wire form is the identity.
            assert_eq!(serde_json::to_string(&parsed).expect("serializes"), line, "{message:?}");
        }
    }

    /// Hooks installed before other agents were supported send neither `agent` nor `message`.
    #[test]
    fn a_hook_without_agent_or_message_is_claudes() {
        let parsed: Message =
            serde_json::from_str(r#"{"kind":"hook","tab":"7","event":"Stop"}"#).expect("parses");
        let Message::Hook { tab, agent, event, message } = parsed else { panic!("not a hook: {parsed:?}") };
        assert_eq!((tab.as_str(), agent, event.as_str(), message), ("7", AgentId::Claude, "Stop", None));
    }
}
