//! The events a stream tells (`wh events`): what happened, to which window and terminal, as the
//! lines of newline-delimited JSON the stream writes after its first reply.
//!
//! Every line is one object with `version`, a `seq` that only grows over the app's run, `at_ms`, the
//! `event` and the ids `list` uses. A client ignores an event kind and a field it does not know: a
//! new kind of event is an addition, not a new protocol version. The stream's own lines
//! ([`Notice`]: heartbeat, lagged, closed) carry no seq, since they are not events.

mod filter;
mod topic;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use filter::{EventFilter, EventsSpec, Subscribed, TERMINALS_MAX, WINDOWS_MAX};
pub use topic::{Topic, Topics};

use super::{PROTOCOL_VERSION, TerminalState};

/// A stream with nothing to tell for this long writes a heartbeat, so a client can tell a quiet
/// app from a dead one (and the app, a client that went away).
pub const HEARTBEAT: std::time::Duration = std::time::Duration::from_secs(15);

/// Something that happened, before the bus numbers and dates it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    /// The window it happened in (the new one, for a terminal that moved).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<i64>,
    /// The terminal it is about, when it is about one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal: Option<i64>,
    #[serde(flatten)]
    pub body: Body,
}

/// What kind of event it is, and what it says.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Body {
    /// A terminal's state changed.
    Status {
        status: TerminalState,
        was: TerminalState,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        agent: Option<String>,
        /// The agent's last words, when it needs you.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
    /// A terminal's title changed (`name` is what its row calls it).
    Title {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        name: String,
    },
    /// A terminal went to another folder.
    Folder {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<String>,
    },
    Bell,
    /// A command started or finished, in a shell with integration.
    Block {
        block: i64,
        phase: BlockPhase,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exit_code: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        took_ms: Option<u64>,
        /// The command line: terminal text, so only subscribers who may read the terminal get it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<String>,
    },
    /// A split was made or changed; empty `terminals` means it is gone.
    Split {
        split: i64,
        terminals: Vec<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        focused: Option<i64>,
    },
    TerminalOpened {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        folder: Option<String>,
    },
    TerminalClosed,
    /// A terminal moved to another window (`from` is the one it left; `window` the one it is in).
    TerminalMoved {
        from: i64,
    },
    /// A terminal was named (`name`) or got its automatic name back (none).
    Renamed {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    WindowOpened,
    WindowClosed,
    /// An event kind a newer Wormhole told.
    #[serde(other)]
    Other,
}

impl Body {
    /// The topic a subscriber must have asked for to hear this; none for a kind this build does not
    /// know.
    pub fn topic(&self) -> Option<Topic> {
        Some(match self {
            Body::Status { .. } => Topic::Status,
            Body::Title { .. } => Topic::Title,
            Body::Folder { .. } => Topic::Folder,
            Body::Bell => Topic::Bell,
            Body::Block { .. } => Topic::Blocks,
            Body::Split { .. } => Topic::Splits,
            Body::TerminalOpened { .. }
            | Body::TerminalClosed
            | Body::TerminalMoved { .. }
            | Body::Renamed { .. } => Topic::Terminals,
            Body::WindowOpened | Body::WindowClosed => Topic::Windows,
            Body::Other => return None,
        })
    }

    /// The event's name on the wire.
    pub fn name(&self) -> &'static str {
        match self {
            Body::Status { .. } => "status",
            Body::Title { .. } => "title",
            Body::Folder { .. } => "folder",
            Body::Bell => "bell",
            Body::Block { .. } => "block",
            Body::Split { .. } => "split",
            Body::TerminalOpened { .. } => "terminal_opened",
            Body::TerminalClosed => "terminal_closed",
            Body::TerminalMoved { .. } => "terminal_moved",
            Body::Renamed { .. } => "renamed",
            Body::WindowOpened => "window_opened",
            Body::WindowClosed => "window_closed",
            Body::Other => "other",
        }
    }
}

impl Event {
    /// Whether some of the event is terminal text, which only a subscriber who may read that
    /// terminal gets (the rest is what `list` tells anyone).
    pub fn has_gated_fields(&self) -> bool {
        matches!(self.body, Body::Block { command: Some(_), .. })
    }

    /// The event as a subscriber without the grant to read its terminal gets it.
    #[must_use]
    pub fn without_gated_fields(&self) -> Event {
        let mut event = self.clone();
        if let Body::Block { command, .. } = &mut event.body {
            *command = None;
        }
        event
    }
}

/// A command's moment: it began, or it ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockPhase {
    Started,
    Finished,
    #[serde(other)]
    Other,
}

/// An event as a stream writes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventLine {
    pub version: u32,
    pub seq: u64,
    pub at_ms: u64,
    #[serde(flatten)]
    pub event: Event,
}

/// Why a stream was closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    /// What it was granted is no longer allowed.
    NotAllowed,
    /// It did not read what it was sent.
    TooSlow,
    /// Wormhole is quitting.
    Quitting,
    /// Some limit was hit.
    Limit,
    #[serde(other)]
    Other,
}

/// A line of the stream that is not an event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Notice {
    /// Still here: nothing happened for a while. `seq` is the last one sent.
    Heartbeat { seq: u64 },
    /// The stream fell behind and `dropped` events were not kept for it.
    Lagged { dropped: u64 },
    /// The stream ends.
    Closed { reason: CloseReason },
}

/// A [`Notice`] as it is written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoticeLine {
    pub version: u32,
    #[serde(flatten)]
    pub notice: Notice,
}

impl NoticeLine {
    pub fn new(notice: Notice) -> Self {
        Self { version: PROTOCOL_VERSION, notice }
    }
}

/// One line of a stream, read.
#[derive(Debug, Clone, PartialEq)]
pub enum StreamLine {
    Event(EventLine),
    Notice(Notice),
}

impl StreamLine {
    /// Read a line; `None` when it is not one this build can use (a client skips it).
    pub fn parse(line: &str) -> Option<StreamLine> {
        let value: Value = serde_json::from_str(line).ok()?;
        let kind = value.get("event")?.as_str()?;
        if matches!(kind, "heartbeat" | "lagged" | "closed") {
            return serde_json::from_value::<NoticeLine>(value).ok().map(|n| StreamLine::Notice(n.notice));
        }
        serde_json::from_value::<EventLine>(value).ok().map(StreamLine::Event)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BlockPhase, Body, CloseReason, Event, EventLine, Notice, NoticeLine, StreamLine, TerminalState, Topic,
    };

    fn line(seq: u64, event: Event) -> EventLine {
        EventLine { version: 1, seq, at_ms: 1_793_023_327_120, event }
    }

    /// The lines of the design's examples, as the stream writes them.
    #[test]
    fn events_serialize_to_the_documented_lines_and_back() {
        let cases = [
            (
                line(
                    431,
                    Event {
                        window: Some(1),
                        terminal: Some(3),
                        body: Body::Status {
                            status: TerminalState::NeedsYou,
                            was: TerminalState::Working,
                            agent: Some("claude".into()),
                            message: Some("Allow Bash?".into()),
                        },
                    },
                ),
                r#"{"version":1,"seq":431,"at_ms":1793023327120,"window":1,"terminal":3,"event":"status","status":"needs_you","was":"working","agent":"claude","message":"Allow Bash?"}"#,
            ),
            (
                line(
                    432,
                    Event {
                        window: Some(1),
                        terminal: Some(8),
                        body: Body::Block {
                            block: 57,
                            phase: BlockPhase::Finished,
                            exit_code: Some(101),
                            took_ms: Some(3512),
                            command: Some("cargo test".into()),
                        },
                    },
                ),
                r#"{"version":1,"seq":432,"at_ms":1793023327120,"window":1,"terminal":8,"event":"block","block":57,"phase":"finished","exit_code":101,"took_ms":3512,"command":"cargo test"}"#,
            ),
            (
                line(
                    434,
                    Event {
                        window: Some(1),
                        terminal: None,
                        body: Body::Split { split: 12, terminals: vec![3, 9], focused: Some(9) },
                    },
                ),
                r#"{"version":1,"seq":434,"at_ms":1793023327120,"window":1,"event":"split","split":12,"terminals":[3,9],"focused":9}"#,
            ),
            (
                line(
                    435,
                    Event {
                        window: Some(2),
                        terminal: Some(10),
                        body: Body::TerminalOpened { folder: Some("/Users/me/src/api".into()) },
                    },
                ),
                r#"{"version":1,"seq":435,"at_ms":1793023327120,"window":2,"terminal":10,"event":"terminal_opened","folder":"/Users/me/src/api"}"#,
            ),
            (
                line(436, Event { window: Some(1), terminal: Some(3), body: Body::Bell }),
                r#"{"version":1,"seq":436,"at_ms":1793023327120,"window":1,"terminal":3,"event":"bell"}"#,
            ),
            (
                line(
                    437,
                    Event { window: Some(2), terminal: Some(3), body: Body::TerminalMoved { from: 1 } },
                ),
                r#"{"version":1,"seq":437,"at_ms":1793023327120,"window":2,"terminal":3,"event":"terminal_moved","from":1}"#,
            ),
        ];
        for (expected, text) in cases {
            assert_eq!(serde_json::to_string(&expected).expect("serializes"), text);
            assert_eq!(StreamLine::parse(text), Some(StreamLine::Event(expected)));
        }
    }

    #[test]
    fn the_streams_own_lines_have_no_seq() {
        let cases = [
            (Notice::Heartbeat { seq: 430 }, r#"{"version":1,"event":"heartbeat","seq":430}"#),
            (Notice::Lagged { dropped: 12 }, r#"{"version":1,"event":"lagged","dropped":12}"#),
            (
                Notice::Closed { reason: CloseReason::TooSlow },
                r#"{"version":1,"event":"closed","reason":"too_slow"}"#,
            ),
        ];
        for (notice, text) in cases {
            assert_eq!(serde_json::to_string(&NoticeLine::new(notice.clone())).expect("serializes"), text);
            assert_eq!(StreamLine::parse(text), Some(StreamLine::Notice(notice)));
        }
    }

    /// A newer app may add event kinds, fields and reasons; this client still reads the line.
    #[test]
    fn a_line_from_a_newer_app_still_reads() {
        let unknown = r#"{"version":1,"seq":9,"at_ms":1,"window":1,"event":"teleported","where":"mars"}"#;
        let Some(StreamLine::Event(read)) = StreamLine::parse(unknown) else { panic!("not read") };
        assert_eq!(read.event.body, Body::Other);
        assert_eq!(read.event.body.topic(), None);
        let extra = r#"{"version":1,"seq":9,"at_ms":1,"window":1,"terminal":2,"event":"bell","loud":true}"#;
        assert!(matches!(StreamLine::parse(extra), Some(StreamLine::Event(_))));
        let closed = r#"{"version":1,"event":"closed","reason":"cosmic_rays"}"#;
        assert_eq!(
            StreamLine::parse(closed),
            Some(StreamLine::Notice(Notice::Closed { reason: CloseReason::Other }))
        );
        assert_eq!(StreamLine::parse("not json"), None);
        assert_eq!(StreamLine::parse(r#"{"version":1}"#), None);
        assert_eq!(StreamLine::parse(r#"{"version":1,"ok":true}"#), None, "a reply is not a stream line");
    }

    #[test]
    fn every_event_belongs_to_a_topic_and_has_a_name() {
        let bodies = [
            (Body::Bell, Topic::Bell, "bell"),
            (Body::TerminalClosed, Topic::Terminals, "terminal_closed"),
            (Body::WindowOpened, Topic::Windows, "window_opened"),
            (Body::WindowClosed, Topic::Windows, "window_closed"),
            (Body::Renamed { name: None }, Topic::Terminals, "renamed"),
            (Body::Folder { folder: None }, Topic::Folder, "folder"),
            (Body::Title { title: None, name: "x".into() }, Topic::Title, "title"),
        ];
        for (body, topic, name) in bodies {
            assert_eq!((body.topic(), body.name()), (Some(topic), name));
            // The name is what the line says.
            let text =
                serde_json::to_string(&Event { window: None, terminal: None, body }).expect("serializes");
            assert!(text.contains(&format!(r#""event":"{name}""#)), "{text}");
        }
    }

    #[test]
    fn only_a_blocks_command_is_terminal_text() {
        let block = Event {
            window: Some(1),
            terminal: Some(2),
            body: Body::Block {
                block: 1,
                phase: BlockPhase::Started,
                exit_code: None,
                took_ms: None,
                command: Some("rm -rf /".into()),
            },
        };
        assert!(block.has_gated_fields());
        let public = block.without_gated_fields();
        assert!(!public.has_gated_fields());
        let text = serde_json::to_string(&public).expect("serializes");
        assert!(!text.contains("rm -rf"), "{text}");
        assert!(!Event { window: None, terminal: None, body: Body::Bell }.has_gated_fields());
    }
}
