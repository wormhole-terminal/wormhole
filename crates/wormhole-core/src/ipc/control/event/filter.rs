//! What a subscriber asks for, and the answer it gets first.

use serde::{Deserialize, Serialize};

use super::{Event, Topic, Topics};
use crate::ipc::control::{Error, ErrorCode, Listing};

/// How many terminals one subscription may be limited to.
pub const TERMINALS_MAX: usize = 64;
/// How many windows one subscription may be limited to.
pub const WINDOWS_MAX: usize = 16;

/// The fields of an `events` request, as they travel.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventsSpec {
    /// Topic names; none means the default set (everything but `output`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
    /// Only these terminals' events (and none of the windows').
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terminals: Vec<i64>,
    /// Only these windows' events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub windows: Vec<i64>,
    /// Add `output` to the topics.
    #[serde(default)]
    pub output: bool,
}

impl EventsSpec {
    /// The request as a filter, or what is wrong with it: an unknown topic, too many ids.
    pub fn filter(&self) -> Result<EventFilter, Error> {
        let mut topics = Topics::default();
        for name in &self.topics {
            let topic = Topic::from_name(name).ok_or_else(|| {
                // The name is the caller's: repeated back only when it is a short plain word.
                let plain = name.len() <= 32 && name.bytes().all(|b| b.is_ascii_lowercase() || b == b'_');
                let shown = if plain { name.as_str() } else { "?" };
                Error::new(
                    ErrorCode::BadRequest,
                    format!("unknown topic `{shown}` (the topics are {})", Topics::all().names().join(", ")),
                )
            })?;
            topics = topics.with(topic);
        }
        if topics.is_empty() {
            topics = Topics::default_set();
        }
        if self.output {
            topics = topics.with(Topic::Output);
        }
        if self.terminals.len() > TERMINALS_MAX {
            return Err(Error::new(
                ErrorCode::BadRequest,
                format!("events takes at most {TERMINALS_MAX} terminals, not {}", self.terminals.len()),
            ));
        }
        if self.windows.len() > WINDOWS_MAX {
            return Err(Error::new(
                ErrorCode::BadRequest,
                format!("events takes at most {WINDOWS_MAX} windows, not {}", self.windows.len()),
            ));
        }
        Ok(EventFilter { topics, terminals: self.terminals.clone(), windows: self.windows.clone() })
    }
}

/// Which events a subscriber hears: the topics, and, when named, only these terminals and windows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventFilter {
    pub topics: Topics,
    pub terminals: Vec<i64>,
    pub windows: Vec<i64>,
}

impl EventFilter {
    /// Whether the filter lets `event` through.
    pub fn matches(&self, event: &Event) -> bool {
        let Some(topic) = event.body.topic() else { return false };
        self.topics.contains(topic)
            && (self.terminals.is_empty() || event.terminal.is_some_and(|id| self.terminals.contains(&id)))
            && (self.windows.is_empty() || event.window.is_some_and(|id| self.windows.contains(&id)))
    }
}

/// The reply to `events`: the state as of `seq`, so a client starts from a snapshot and every later
/// change has a higher seq.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subscribed {
    pub subscribed: bool,
    /// The last seq before this subscription: every event it gets is later.
    pub seq: u64,
    /// Random for each run of the app: seqs start again with it.
    pub boot: String,
    pub topics: Vec<String>,
    pub listing: Listing,
}

#[cfg(test)]
mod tests {
    use super::{EventsSpec, TERMINALS_MAX, WINDOWS_MAX};
    use crate::ipc::control::ErrorCode;
    use crate::ipc::control::event::{Body, Event, Topic};

    fn spec(topics: &[&str]) -> EventsSpec {
        EventsSpec { topics: topics.iter().map(|t| t.to_string()).collect(), ..EventsSpec::default() }
    }

    fn bell(window: i64, terminal: i64) -> Event {
        Event { window: Some(window), terminal: Some(terminal), body: Body::Bell }
    }

    #[test]
    fn no_topics_means_all_but_output_and_output_adds_it() {
        let filter = spec(&[]).filter().expect("valid");
        assert!(filter.topics.contains(Topic::Status) && !filter.topics.contains(Topic::Output));
        let with = EventsSpec { output: true, ..spec(&[]) }.filter().expect("valid");
        assert!(with.topics.contains(Topic::Output) && with.topics.contains(Topic::Bell));
        let named = spec(&["bell", "output"]).filter().expect("valid");
        assert_eq!(named.topics.names(), ["bell", "output"]);
    }

    #[test]
    fn an_unknown_topic_and_too_many_ids_are_bad_requests() {
        let error = spec(&["status", "nope"]).filter().expect_err("refused");
        assert_eq!(error.code, ErrorCode::BadRequest);
        assert!(error.message.contains("`nope`") && error.message.contains("windows"), "{}", error.message);
        let hostile = spec(&["\u{1b}[31m"]).filter().expect_err("refused");
        assert!(hostile.message.contains("`?`") && !hostile.message.contains('\u{1b}'));
        let many = EventsSpec { terminals: (0..=TERMINALS_MAX as i64).collect(), ..spec(&[]) };
        assert_eq!(many.filter().expect_err("refused").code, ErrorCode::BadRequest);
        let at_cap = EventsSpec { windows: (0..WINDOWS_MAX as i64).collect(), ..spec(&[]) };
        assert!(at_cap.filter().is_ok());
        let over = EventsSpec { windows: (0..=WINDOWS_MAX as i64).collect(), ..spec(&[]) };
        assert_eq!(over.filter().expect_err("refused").code, ErrorCode::BadRequest);
    }

    #[test]
    fn the_filter_lets_through_its_topics_terminals_and_windows() {
        let all = spec(&[]).filter().expect("valid");
        assert!(all.matches(&bell(1, 3)));
        let only_status = spec(&["status"]).filter().expect("valid");
        assert!(!only_status.matches(&bell(1, 3)), "another topic");
        let terminal = EventsSpec { terminals: vec![3], ..spec(&[]) }.filter().expect("valid");
        assert!(terminal.matches(&bell(1, 3)) && !terminal.matches(&bell(1, 4)));
        let window_event = Event { window: Some(1), terminal: None, body: Body::WindowOpened };
        assert!(!terminal.matches(&window_event), "a window's event has no terminal");
        let window = EventsSpec { windows: vec![2], ..spec(&[]) }.filter().expect("valid");
        assert!(window.matches(&bell(2, 9)) && !window.matches(&bell(1, 9)));
        assert!(window.matches(&Event { window: Some(2), terminal: None, body: Body::WindowOpened }));
        // An event kind this build doesn't know is never passed on.
        let other = Event { window: None, terminal: None, body: Body::Other };
        assert!(!all.matches(&other));
    }
}
