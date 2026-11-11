//! A program driving a terminal says so on the chip: what the request did, who did it, and when nothing is said.

use std::io::Write as _;

use serde_json::json;
use wormhole_core::storage::paths::DataRoot;

use super::support::{eventually, fixture};
use crate::hook_server::HookEvent;

fn driven(events: Vec<HookEvent>) -> Vec<(i64, bool)> {
    events
        .into_iter()
        .filter_map(|event| match event {
            HookEvent::Driven { terminal, typing, .. } => Some((terminal, typing)),
            _ => None,
        })
        .collect()
}

#[test]
fn typing_and_reading_are_told_after_the_handler_said_yes() {
    let f = fixture("open");
    let id = f.terminal();
    let mut client = f.client();
    let target = json!({"kind": "terminal", "id": id});
    client.control(json!({"action": "send_text", "target": target, "text": "hi", "submit": true}));
    client.control(json!({"action": "read", "target": target}));
    // Not a driver: a listing, nor a request that was refused (no such terminal).
    client.control(json!({"action": "list"}));
    client.control(json!({"action": "send_text", "target": {"kind": "terminal", "id": 99}, "text": "x"}));
    eventually("both told", || f.wakes.count() > 0);
    assert_eq!(driven(f.events()), vec![(id, true), (id, false)]);
}

#[test]
fn nothing_is_told_where_the_chip_is_off() {
    let f = fixture("open");
    let id = f.terminal();
    let config = json!({
        "agent_access": "open", "control_write": true,
        "feature_flags": { "automation": true, "notices": false }
    });
    std::fs::File::create(DataRoot::new(&f.dir).default_profile().config_file())
        .and_then(|mut file| file.write_all(config.to_string().as_bytes()))
        .expect("config");
    f.server.settings_changed("default".into());
    let mut client = f.client();
    client.control(json!({"action": "send_text", "target": {"kind": "terminal", "id": id}, "text": "hi"}));
    assert_eq!(driven(f.events()), vec![]);
}
