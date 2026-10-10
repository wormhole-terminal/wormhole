//! Runs `protocol-vectors.json`, the examples of the agent status protocol, against the
//! code: the wire form of every message, which lines the server ignores, what an event is worth
//! and what a hook's payload adds to it. When the spec and the code disagree, this fails.

use serde_json::Value;
use wormhole_core::agents::AgentId;
use wormhole_core::agents::integrations::{Signal, describe, signal};
use wormhole_core::ipc::Message;

fn vectors() -> Value {
    serde_json::from_str(include_str!("protocol-vectors.json")).expect("the vectors are JSON")
}

fn list<'a>(vectors: &'a Value, key: &str) -> &'a Vec<Value> {
    vectors[key].as_array().unwrap_or_else(|| panic!("{key} is a list"))
}

fn text<'a>(entry: &'a Value, key: &str) -> &'a str {
    entry[key].as_str().unwrap_or_else(|| panic!("{key} is a string in {entry}"))
}

fn roundtrip(line: &str) -> Option<String> {
    let message: Message = serde_json::from_str(line).ok()?;
    serde_json::to_string(&message).ok()
}

#[test]
fn every_message_has_the_wire_form_the_spec_gives() {
    let vectors = vectors();
    for entry in list(&vectors, "messages") {
        let line = text(entry, "line");
        assert_eq!(roundtrip(line).as_deref(), Some(line));
    }
}

#[test]
fn older_lines_are_accepted_as_what_the_spec_says() {
    let vectors = vectors();
    for entry in list(&vectors, "messages_accepted_as") {
        assert_eq!(roundtrip(text(entry, "line")).as_deref(), Some(text(entry, "becomes")));
    }
}

#[test]
fn the_lines_the_spec_says_are_ignored_do_not_parse() {
    let vectors = vectors();
    for line in list(&vectors, "messages_ignored") {
        let line = line.as_str().expect("a line");
        assert!(serde_json::from_str::<Message>(line).is_err(), "{line} parses");
    }
}

#[test]
fn every_event_is_worth_what_the_spec_says() {
    let vectors = vectors();
    for entry in list(&vectors, "signals") {
        let agent: AgentId = serde_json::from_value(entry["agent"].clone()).expect("a known agent id");
        let event = text(entry, "event");
        let got = signal(agent, event).map(|signal| match signal {
            Signal::Busy => "busy",
            Signal::Attention => "attention",
            Signal::Done => "done",
            Signal::Quiet => "quiet",
            Signal::Ended => "ended",
        });
        assert_eq!(got, entry["signal"].as_str(), "{agent:?} {event}");
    }
}

#[test]
fn a_payload_adds_what_the_spec_says_to_its_event() {
    let vectors = vectors();
    for entry in list(&vectors, "describe") {
        let (event, message) = describe(text(entry, "event"), &entry["payload"]);
        assert_eq!(event, text(entry, "expect_event"));
        assert_eq!(message.as_deref(), entry["expect_message"].as_str(), "{event}");
    }
}
