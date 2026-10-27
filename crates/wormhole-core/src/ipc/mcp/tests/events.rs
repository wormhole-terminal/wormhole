//! `wait_events`: a cursor over the stream of the app's events, kept between calls.

use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};

use super::{Behavior, Client, FakeApp, listing, until};
use crate::ipc::control::{Action, Error, ErrorCode, Reply};

/// An app whose event streams are the channels handed out in order: each `events` request takes the next.
fn app_with_streams(boots: &[&str], first_seq: u64) -> (FakeApp, Vec<Sender<String>>) {
    let mut senders = Vec::new();
    let mut receivers = Vec::new();
    for _ in boots {
        let (tx, rx) = channel();
        senders.push(tx);
        receivers.push(rx);
    }
    let receivers = Mutex::new(receivers.into_iter());
    let boots: Vec<String> = boots.iter().map(ToString::to_string).collect();
    let started = Arc::new(Mutex::new(0usize));
    let app = FakeApp::start(move |request| match &request.action {
        Action::Events(_) => {
            let mut started = started.lock().expect("lock");
            let boot = boots[*started].clone();
            *started += 1;
            let rx = receivers.lock().expect("lock").next().expect("a stream per subscription");
            Behavior::Stream {
                reply: Reply::ok(json!({"subscribed": true, "seq": first_seq, "boot": boot, "topics": [],
                                        "listing": {"windows": []}})),
                lines: rx,
            }
        }
        Action::List => Behavior::Reply(Reply::ok(listing())),
        _ => Behavior::Reply(Error::new(ErrorCode::Unsupported, "not in this test").into()),
    });
    (app, senders)
}

fn status(seq: u64, terminal: u64, to: &str) -> String {
    json!({"version": 1, "seq": seq, "at_ms": 1, "window": 1, "terminal": terminal, "event": "status",
           "status": to, "was": "working", "agent": "claude", "message": "Allow Bash?"})
    .to_string()
}

fn bell(seq: u64, terminal: u64) -> String {
    json!({"version": 1, "seq": seq, "at_ms": 1, "window": 1, "terminal": terminal, "event": "bell"})
        .to_string()
}

fn events(client: &mut Client, arguments: Value) -> Value {
    let response = client.tool("wait_events", arguments);
    assert_eq!(response["result"]["isError"], json!(false), "{response}");
    response["result"]["structuredContent"].clone()
}

/// Call until the stream has delivered `count` events past `since` (the thread that drains it runs on
/// its own time).
fn events_after(client: &mut Client, since: u64, boot: &str, count: usize, extra: Value) -> Value {
    let mut arguments = json!({"since_seq": since, "boot": boot, "timeout_seconds": 0});
    arguments.as_object_mut().expect("object").extend(extra.as_object().expect("object").clone());
    let mut got = Value::Null;
    until(
        || {
            got = events(client, arguments.clone());
            got["events"].as_array().expect("events").len() >= count
        },
        "the events to arrive",
    );
    got
}

#[test]
fn the_first_call_starts_the_stream_and_gives_a_snapshot_and_a_cursor() {
    let (app, _streams) = app_with_streams(&["b1"], 430);
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let first = events(&mut client, json!({}));
    assert_eq!((first["boot"].clone(), first["seq"].clone()), (json!("b1"), json!(430)));
    assert_eq!(
        (first["events"].clone(), first["more"].clone(), first["gap"].clone()),
        (json!([]), json!(false), json!(false))
    );
    assert_eq!(first["snapshot"], listing());
    // The subscription is the command line's: from this tab, the default topics, no output.
    let requests = app.requests();
    assert_eq!(requests[0], json!({"version": 1, "from": "7", "action": "events", "output": false}));
    assert_eq!(client.finish(), 0);
}

#[test]
fn later_calls_return_what_came_after_the_cursor_and_filter_it() {
    let (app, streams) = app_with_streams(&["b1"], 430);
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    events(&mut client, json!({}));
    streams[0].send(status(431, 3, "needs_you")).expect("send");
    streams[0].send(bell(432, 4)).expect("send");
    streams[0].send(status(433, 4, "finished")).expect("send");

    let all = events_after(&mut client, 430, "b1", 3, json!({}));
    assert_eq!(all["seq"], json!(433));
    let seqs: Vec<_> = all["events"].as_array().expect("events").iter().map(|e| e["seq"].clone()).collect();
    assert_eq!(seqs, [json!(431), json!(432), json!(433)]);
    assert_eq!(all["events"][0]["event"], json!("status"));
    assert_eq!(all["events"][0]["status"], json!("needs_you"));
    assert_eq!(all["gap"], json!(false));
    assert!(all.get("snapshot").is_none(), "a snapshot only without a cursor or after a gap");

    // Filtered by topic and by terminal, and the cursor still moves to the end of what was heard.
    let bells = events_after(&mut client, 430, "b1", 1, json!({"topics": ["bell"]}));
    assert_eq!(bells["events"].as_array().expect("events").len(), 1);
    assert_eq!(bells["seq"], json!(433));
    let terminal_three = events_after(&mut client, 430, "b1", 1, json!({"terminals": [3]}));
    assert_eq!(terminal_three["events"].as_array().expect("events").len(), 1);

    // The text a client that does not read structured content gets names the events.
    let response = client.tool("wait_events", json!({"since_seq": 430, "boot": "b1", "timeout_seconds": 0}));
    let text = response["result"]["content"][0]["text"].as_str().expect("text");
    assert!(text.contains("3 events") && text.contains("needs") && text.contains("claude"), "{text}");
    assert_eq!(client.finish(), 0);
}

#[test]
fn max_events_cuts_the_batch_and_the_cursor_stops_at_the_last_one_returned() {
    let (app, streams) = app_with_streams(&["b1"], 10);
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    events(&mut client, json!({}));
    for seq in 11..=15 {
        streams[0].send(bell(seq, 3)).expect("send");
    }
    events_after(&mut client, 10, "b1", 5, json!({}));
    let page =
        events(&mut client, json!({"since_seq": 10, "boot": "b1", "max_events": 2, "timeout_seconds": 0}));
    assert_eq!((page["more"].clone(), page["seq"].clone()), (json!(true), json!(12)));
    let next =
        events(&mut client, json!({"since_seq": 12, "boot": "b1", "max_events": 10, "timeout_seconds": 0}));
    assert_eq!((next["more"].clone(), next["seq"].clone()), (json!(false), json!(15)));
    assert_eq!(next["events"].as_array().expect("events").len(), 3);
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_call_waits_for_the_first_event_up_to_its_timeout() {
    let (app, streams) = app_with_streams(&["b1"], 10);
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    events(&mut client, json!({}));
    let started = std::time::Instant::now();
    let none = events(&mut client, json!({"since_seq": 10, "boot": "b1", "timeout_seconds": 1}));
    assert!(started.elapsed() >= std::time::Duration::from_millis(900));
    assert_eq!(
        (none["events"].clone(), none["seq"].clone(), none["gap"].clone()),
        (json!([]), json!(10), json!(false))
    );
    // And is let go the moment one arrives.
    let id = client.request(
        "tools/call",
        json!({"name": "wait_events", "arguments": {"since_seq": 10, "boot": "b1", "timeout_seconds": 20}}),
    );
    std::thread::sleep(std::time::Duration::from_millis(200));
    streams[0].send(bell(11, 3)).expect("send");
    let woken = client.read();
    assert_eq!(woken["id"], json!(id));
    assert_eq!(woken["result"]["structuredContent"]["seq"], json!(11));
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_cursor_the_buffer_cannot_honour_is_a_gap_with_a_snapshot() {
    let (app, streams) = app_with_streams(&["b1"], 430);
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    events(&mut client, json!({}));
    streams[0].send(bell(431, 3)).expect("send");
    events_after(&mut client, 430, "b1", 1, json!({}));
    // From before the stream began: events in between were not recorded.
    let old = events(&mut client, json!({"since_seq": 5, "boot": "b1", "timeout_seconds": 0}));
    assert_eq!(old["gap"], json!(true));
    assert_eq!(old["snapshot"], listing());
    // From another run of the app: the cursor means nothing.
    let other = events(&mut client, json!({"since_seq": 431, "boot": "elsewhere", "timeout_seconds": 0}));
    assert_eq!(
        (other["gap"].clone(), other["boot"].clone(), other["seq"].clone()),
        (json!(true), json!("b1"), json!(431))
    );
    assert_eq!(other["events"], json!([]));
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_stream_that_fell_behind_is_a_gap_after_the_cursor() {
    let (app, streams) = app_with_streams(&["b1"], 100);
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    events(&mut client, json!({}));
    streams[0].send(bell(101, 3)).expect("send");
    streams[0].send(json!({"version": 1, "event": "lagged", "dropped": 5}).to_string()).expect("send");
    streams[0].send(bell(108, 3)).expect("send");
    let got = events_after(&mut client, 101, "b1", 1, json!({}));
    assert_eq!(got["events"][0]["seq"], json!(108));
    assert_eq!(got["gap"], json!(true), "events 102 to 107 were dropped on the way");
    // A cursor at the last event before the loss is not told of one it is not past.
    let after = events(&mut client, json!({"since_seq": 108, "boot": "b1", "timeout_seconds": 0}));
    assert_eq!(after["gap"], json!(false));
    assert_eq!(client.finish(), 0);
}

#[test]
fn when_the_stream_ends_the_call_says_so_and_the_next_one_starts_it_again() {
    let (app, streams) = app_with_streams(&["b1", "b2"], 10);
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    events(&mut client, json!({}));
    streams[0]
        .send(json!({"version": 1, "event": "closed", "reason": "quitting"}).to_string())
        .expect("send");
    let mut closed = Value::Null;
    until(
        || {
            closed = events(&mut client, json!({"since_seq": 10, "boot": "b1", "timeout_seconds": 0}));
            closed.get("closed").is_some()
        },
        "the stream to end",
    );
    assert_eq!(closed["closed"], json!("quitting"));
    // The next call subscribes again; the old cursor belongs to the old run.
    let again = events(&mut client, json!({"since_seq": 10, "boot": "b1", "timeout_seconds": 0}));
    assert_eq!((again["boot"].clone(), again["gap"].clone()), (json!("b2"), json!(true)));
    assert!(again["snapshot"].is_object());
    let subscriptions = app.requests().iter().filter(|r| r["action"] == "events").count();
    assert_eq!(subscriptions, 2);
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_subscription_the_app_refuses_is_an_error_result_and_the_next_call_tries_again() {
    let refusing = FakeApp::start(|request| match request.action {
        Action::Events(_) => Behavior::Reply(Error::new(ErrorCode::NotAllowed, "reading is off").into()),
        _ => Behavior::Reply(Reply::ok(listing())),
    });
    let mut client = Client::with_app(&refusing);
    client.initialize("2025-11-25");
    for _ in 0..2 {
        let result = client.tool("wait_events", json!({"output": true}))["result"].clone();
        assert_eq!(result["isError"], json!(true));
        assert!(result["content"][0]["text"].as_str().expect("text").contains("reading is off"));
    }
    let asked: Vec<_> = refusing.requests().into_iter().filter(|r| r["action"] == "events").collect();
    assert_eq!(asked.len(), 2);
    assert_eq!(asked[0]["output"], json!(true), "output has a subscription of its own");
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_cancelled_wait_for_events_gets_no_response() {
    let (app, _streams) = app_with_streams(&["b1"], 10);
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    events(&mut client, json!({}));
    let id = client.request(
        "tools/call",
        json!({"name": "wait_events", "arguments": {"since_seq": 10, "boot": "b1", "timeout_seconds": 30}}),
    );
    std::thread::sleep(std::time::Duration::from_millis(200));
    client.notify("notifications/cancelled", json!({"requestId": id}));
    assert_eq!(client.call("ping", json!({}))["result"], json!({}));
    assert_eq!(client.finish(), 0);
}
