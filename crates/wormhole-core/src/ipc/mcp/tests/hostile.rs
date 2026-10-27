//! Whatever arrives on stdin: the server answers what it can, never panics, writes nothing but protocol.

use serde_json::json;

use super::{Client, FakeApp};
use crate::ipc::control::Reply;

fn app() -> FakeApp {
    FakeApp::always(Reply::ok(json!({"lines": ["\u{1b}[31mred\u{202e}"], "terminal": 3})))
}

#[test]
fn what_is_not_json_is_a_parse_error_and_the_server_goes_on() {
    let app = app();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    for junk in [&b"{nope\n"[..], b"\xff\xfe\n", b"\n  \n", b"null\n", b"7\n", b"\"text\"\n"] {
        client.send_raw(junk);
    }
    // Blank lines are skipped; each of the others is answered with an error and a null id.
    let mut errors = Vec::new();
    for _ in 0..5 {
        errors.push(client.read());
    }
    assert!(errors.iter().all(|e| e["id"].is_null() && e["error"]["code"].is_i64()), "{errors:?}");
    assert_eq!(errors[0]["error"]["code"], json!(-32700));
    assert_eq!(errors[1]["error"]["code"], json!(-32700));
    assert_eq!(errors[2]["error"]["code"], json!(-32600));
    assert_eq!(client.call("ping", json!({}))["result"], json!({}));
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_message_over_the_cap_is_refused_and_the_next_one_is_served() {
    let app = app();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let mut big = vec![b'x'; (1 << 20) + 500];
    big.push(b'\n');
    client.send_raw(&big);
    let refused = client.read();
    assert_eq!(refused["error"]["code"], json!(-32600));
    assert!(refused["id"].is_null());
    assert_eq!(client.call("ping", json!({}))["result"], json!({}));
    // One that fits is served: a message of nearly the cap, in a string argument.
    let near = "y".repeat((1 << 20) - 200);
    let line = json!({"jsonrpc": "2.0", "id": 99, "method": "tools/call",
                      "params": {"name": "send_text", "arguments": {"terminal": 3, "text": near}}});
    client.send(&line);
    let answer = client.read();
    assert_eq!(answer["id"], json!(99));
    // (Too long for the schema, so it is an error result: but it was read and answered.)
    assert!(answer["result"]["isError"].is_boolean());
    assert_eq!(client.finish(), 0);
}

#[test]
fn deep_nesting_does_not_overflow_the_stack() {
    let app = app();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let mut line = "[".repeat(100_000);
    line.push('\n');
    client.send_raw(line.as_bytes());
    assert_eq!(client.read()["error"]["code"], json!(-32700));
    let mut nested = json!(1);
    for _ in 0..500 {
        nested = json!({"a": nested});
    }
    client.send(&json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "list_terminals", "arguments": nested}}));
    let answer = client.read();
    assert!(answer["id"].as_i64().unwrap_or(0) == 5 || answer["id"].is_null(), "{answer}");
    assert_eq!(client.call("ping", json!({}))["result"], json!({}));
    assert_eq!(client.finish(), 0);
}

#[test]
fn terminal_text_in_a_result_is_escaped_on_the_wire() {
    let app = app();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    client.request("tools/call", json!({"name": "read_terminal", "arguments": {"terminal": 3}}));
    let raw = client.read_raw();
    assert!(
        !raw.contains('\u{1b}') && !raw.contains('\u{202e}'),
        "no control or bidi character is written raw: {raw:?}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&raw).expect("JSON");
    // The structured content reads back as the original; the text is the command line's rendering.
    assert_eq!(parsed["result"]["structuredContent"]["lines"][0], json!("\u{1b}[31mred\u{202e}"));
    assert_eq!(client.finish(), 0);
}

#[test]
fn ids_of_every_allowed_kind_are_answered_with_the_same_id() {
    let app = app();
    let mut client = Client::with_app(&app);
    for id in [json!(0), json!(-4), json!(i64::MAX), json!("a"), json!("")] {
        client.send(&json!({"jsonrpc": "2.0", "id": id, "method": "ping"}));
        assert_eq!(client.read()["id"], id);
    }
    // Ids that are not allowed are refused with a null id.
    for id in [json!(null), json!(1.5), json!([1]), json!({})] {
        client.send(&json!({"jsonrpc": "2.0", "id": id, "method": "ping"}));
        let answer = client.read();
        assert!(answer["id"].is_null() && answer["error"]["code"] == json!(-32600), "{id}: {answer}");
    }
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_response_from_the_client_is_ignored() {
    let app = app();
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    client.send(&json!({"jsonrpc": "2.0", "id": 1, "result": {}}));
    assert_eq!(client.call("ping", json!({}))["result"], json!({}));
    assert_eq!(client.finish(), 0);
}
