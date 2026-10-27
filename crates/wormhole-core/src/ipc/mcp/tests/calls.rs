//! Long calls: they run beside the loop that reads, they end when cancelled, and only four run at once.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;

use super::{Behavior, Client, FakeApp, until};
use crate::ipc::control::{Action, Reply};
use crate::ipc::mcp::rpc::Out;
use crate::ipc::mcp::server::Ticker;
use crate::ipc::mcp::version::Version;

fn hanging_waits() -> FakeApp {
    FakeApp::start(|request| match request.action {
        Action::Wait(_) | Action::Run(_) => Behavior::Hang,
        _ => Behavior::Reply(Reply::ok(json!({}))),
    })
}

fn wait_args() -> serde_json::Value {
    json!({"name": "wait", "arguments": {"terminal": 3, "status": ["idle"]}})
}

#[test]
fn a_cancelled_wait_ends_its_connection_and_gets_no_response() {
    let app = hanging_waits();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let id = client.request("tools/call", wait_args());
    app.wait_for_requests(1);
    client.notify("notifications/cancelled", json!({"requestId": id, "reason": "user stopped it"}));
    // The server goes on: the next response is the ping's, so the cancelled call wrote none.
    let ping = client.call("ping", json!({}));
    assert_eq!(ping["result"], json!({}));
    until(|| app.hung_up() == 1, "the app to see the connection close");
    assert!(client.read_within(Duration::from_millis(300)).is_none(), "nothing came for the cancelled call");
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_cancellation_before_the_call_connects_or_for_an_unknown_id_does_no_harm() {
    let app = hanging_waits();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    client.notify("notifications/cancelled", json!({"requestId": 999}));
    client.notify("notifications/cancelled", json!({}));
    client.notify("notifications/cancelled", json!("not an id"));
    let response = client.call("tools/call", json!({"name": "list_terminals", "arguments": {}}));
    assert_eq!(response["result"]["isError"], json!(false));
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_call_with_a_string_id_is_cancelled_by_that_string_and_not_by_the_number() {
    let app = hanging_waits();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    client.send(&json!({"jsonrpc": "2.0", "id": "7", "method": "tools/call", "params": wait_args()}));
    app.wait_for_requests(1);
    client.notify("notifications/cancelled", json!({"requestId": 7}));
    assert_eq!(client.call("ping", json!({}))["result"], json!({}));
    assert_eq!(app.hung_up(), 0, "the number 7 is not the string \"7\"");
    client.notify("notifications/cancelled", json!({"requestId": "7"}));
    until(|| app.hung_up() == 1, "the connection to close");
    assert_eq!(client.finish(), 0);
}

#[test]
fn the_loop_that_reads_is_never_held_by_a_call() {
    let app = hanging_waits();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let first = client.request("tools/call", wait_args());
    app.wait_for_requests(1);
    // Answered while the wait is still going.
    assert_eq!(client.call("tools/list", json!({}))["result"]["tools"].as_array().expect("tools").len(), 17);
    client.notify("notifications/cancelled", json!({"requestId": first}));
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_fifth_call_is_told_to_wait_at_once() {
    let app = hanging_waits();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let ids: Vec<i64> = (0..4).map(|_| client.request("tools/call", wait_args())).collect();
    app.wait_for_requests(4);
    let fifth = client.call("tools/call", json!({"name": "list_terminals", "arguments": {}}));
    assert_eq!(fifth["result"]["isError"], json!(true));
    assert_eq!(fifth["result"]["_meta"]["app.getwormhole/error"]["code"], json!("rate_limited"));
    assert!(fifth["result"]["content"][0]["text"].as_str().expect("text").contains("four"));
    // A place comes back when a call ends.
    client.notify("notifications/cancelled", json!({"requestId": ids[0]}));
    until(|| app.hung_up() == 1, "the first wait to end");
    let sixth = client.call("tools/call", json!({"name": "list_terminals", "arguments": {}}));
    assert_eq!(sixth["result"]["isError"], json!(false), "{sixth}");
    assert_eq!(client.finish(), 0);
}

#[test]
fn closing_the_input_ends_what_is_still_running_and_the_server_exits() {
    let app = hanging_waits();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    client.request("tools/call", wait_args());
    app.wait_for_requests(1);
    // finish() closes the input: the wait gets its grace, then is ended, and the exit is clean.
    let started = std::time::Instant::now();
    assert_eq!(client.finish(), 0);
    assert!(started.elapsed() < Duration::from_secs(8));
    until(|| app.hung_up() == 1, "the wait to be ended");
}

#[test]
fn a_call_that_finishes_within_the_grace_is_answered_before_the_server_exits() {
    // What `echo '{…}' | wh mcp` expects: the answer to the last line, then the end.
    let app = FakeApp::always(Reply::ok(json!({"tab": 1})));
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let id =
        client.request("tools/call", json!({"name": "send_text", "arguments": {"terminal": 3, "text": "x"}}));
    let answer = client.read();
    assert_eq!(answer["id"], json!(id));
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_long_call_tells_its_progress_while_it_waits_and_stops_when_it_ends() {
    // The ticker alone, with a short interval: the server uses five seconds.
    let (client_end, server_end) = crate::platform::socket::Stream::pair().expect("pair");
    let out = Arc::new(Out::new(server_end));
    let ticker =
        Ticker::start_every(Arc::clone(&out), json!("tok"), Version::V2025_11_25, Duration::from_millis(20));
    let mut reader = std::io::BufReader::new(client_end);
    let mut seen = Vec::new();
    for _ in 0..3 {
        let mut line = String::new();
        std::io::BufRead::read_line(&mut reader, &mut line).expect("a notification");
        seen.push(serde_json::from_str::<serde_json::Value>(&line).expect("JSON"));
    }
    drop(ticker);
    // Nothing follows the end of the call: what was written before it is there to read, and then it is quiet
    // (the thread is gone when `drop` returns).
    reader.get_ref().set_read_timeout(Some(Duration::from_millis(150))).expect("timeout");
    let quiet = |reader: &mut std::io::BufReader<crate::platform::socket::Stream>| {
        let mut line = String::new();
        match std::io::BufRead::read_line(reader, &mut line) {
            Ok(n) => n == 0,
            Err(e) => matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut),
        }
    };
    while !quiet(&mut reader) {}
    assert!(quiet(&mut reader), "no progress after the call ended");
    let progress: Vec<_> = seen.iter().map(|n| n["params"]["progress"].as_u64().expect("number")).collect();
    assert_eq!(progress, [1, 2, 3], "it only grows");
    assert_eq!(seen[0]["method"], json!("notifications/progress"));
    assert_eq!(seen[0]["params"]["progressToken"], json!("tok"));
    assert!(seen[0]["params"]["message"].is_string());
    assert!(seen[0].get("id").is_none(), "a notification has no id");
}
