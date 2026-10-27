//! The handshake, the stateless flow, tools/list and tools/call, as golden exchanges.

use serde_json::{Value, json};

use super::{Client, FakeApp, listing, meta};
use crate::ipc::control::{Error, ErrorCode, Reason, Reply};

fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn names(tools: &Value) -> Vec<&str> {
    tools.as_array().expect("tools").iter().map(|tool| tool["name"].as_str().expect("name")).collect()
}

#[test]
fn initialize_answers_the_clients_version_when_it_is_one_we_speak() {
    let app = FakeApp::always(Reply::ok(json!({})));
    for requested in ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"] {
        let mut client = Client::with_app(&app);
        let result = client.initialize(requested);
        assert_eq!(result["protocolVersion"], json!(requested));
        assert_eq!(result["capabilities"], json!({"tools": {"listChanged": false}}));
        assert!(result["instructions"].as_str().expect("instructions").contains("list_terminals"));
        let info = &result["serverInfo"];
        assert_eq!((info["name"].clone(), info["version"].clone()), (json!("wormhole"), json!(version())));
        // A title is from 2025-06-18 on.
        assert_eq!(info.get("title").is_some(), requested >= "2025-06-18", "{requested}");
        assert_eq!(client.finish(), 0);
    }
}

#[test]
fn initialize_for_a_version_we_do_not_know_answers_with_the_newest_handshake_one() {
    let app = FakeApp::always(Reply::ok(json!({})));
    // Newer than we know, older than any, nonsense, and the stateless one (which has no initialize).
    for requested in ["2099-01-01", "2020-01-01", "latest", "2026-07-28"] {
        let mut client = Client::with_app(&app);
        assert_eq!(client.initialize(requested)["protocolVersion"], json!("2025-11-25"), "{requested}");
        assert_eq!(client.finish(), 0);
    }
    let mut client = Client::with_app(&app);
    let response = client.call("initialize", json!({"capabilities": {}}));
    assert_eq!(response["error"]["code"], json!(-32602));
    assert_eq!(client.finish(), 0);
}

#[test]
fn tools_list_gives_every_tool_with_the_fields_of_the_negotiated_revision() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let listed = client.call("tools/list", json!({}))["result"].clone();
    assert!(listed.get("nextCursor").is_none() && listed.get("ttlMs").is_none());
    let tools = &listed["tools"];
    assert_eq!(tools.as_array().expect("tools").len(), 17);
    let mut sorted = names(tools);
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 17, "names are unique");
    let close = tools.as_array().expect("tools").iter().find(|t| t["name"] == "close").expect("close");
    assert_eq!(close["annotations"]["destructiveHint"], json!(true));
    assert_eq!(close["annotations"]["readOnlyHint"], json!(false));
    assert!(close["outputSchema"].is_object() && close["inputSchema"]["required"] == json!(["terminal"]));
    // The same order every time lets a client cache it.
    let again = client.call("tools/list", json!({}))["result"].clone();
    assert_eq!(again, listed);
    assert_eq!(client.finish(), 0);

    let mut old = Client::with_app(&app);
    old.initialize("2024-11-05");
    let tools = old.call("tools/list", json!({}))["result"]["tools"].clone();
    for tool in tools.as_array().expect("tools") {
        for key in ["annotations", "outputSchema", "title"] {
            assert!(tool.get(key).is_none(), "{} has {key} in 2024-11-05", tool["name"]);
        }
        assert!(tool["inputSchema"].is_object());
    }
    assert_eq!(old.finish(), 0);
}

#[test]
fn tools_list_has_one_page_only() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let response = client.call("tools/list", json!({"cursor": "page-2"}));
    assert_eq!(response["error"]["code"], json!(-32602));
    // An empty or null cursor is the first page.
    assert!(client.call("tools/list", json!({"cursor": null}))["result"]["tools"].is_array());
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_tool_call_makes_the_request_wh_would_and_returns_text_and_structured_content() {
    let app = FakeApp::always(Reply::ok(listing()));
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let response = client.tool("list_terminals", json!({}));
    let result = &response["result"];
    assert_eq!(result["isError"], json!(false));
    assert_eq!(result["structuredContent"], listing());
    let text = result["content"][0]["text"].as_str().expect("text");
    assert!(text.contains("window 1") && text.contains("api"), "{text}");
    assert_eq!(result["content"][0]["type"], json!("text"));
    // The request is the command line's, from this process's tab.
    assert_eq!(app.requests(), [json!({"version": 1, "from": "7", "action": "list"})]);

    let response = client.tool("read_terminal", json!({"terminal": 3, "last_lines": 20}));
    assert_eq!(response["result"]["isError"], json!(false));
    assert_eq!(
        app.requests()[1],
        json!({"version": 1, "from": "7", "action": "read", "target": {"kind": "terminal", "id": 3},
               "range": {"kind": "last", "lines": 20}})
    );
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_tool_call_in_an_earlier_revision_has_the_text_only() {
    let app = FakeApp::always(Reply::ok(json!({"tab": 3})));
    let mut client = Client::with_app(&app);
    client.initialize("2025-03-26");
    let result = client.tool("send_text", json!({"terminal": 3, "text": "ls"}))["result"].clone();
    assert!(result.get("structuredContent").is_none());
    assert_eq!(result["isError"], json!(false));
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_refusal_is_an_error_result_not_a_protocol_error() {
    let refusal: Reply =
        Error::new(ErrorCode::NotAllowed, "typing is off: turn on \"Let the command line type\" in Settings")
            .into();
    let app = FakeApp::always(refusal);
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let response = client.tool("send_text", json!({"terminal": 3, "text": "ls"}));
    assert!(response.get("error").is_none(), "{response}");
    let result = &response["result"];
    assert_eq!(result["isError"], json!(true));
    let text = result["content"][0]["text"].as_str().expect("text");
    assert!(text.contains("typing is off") && text.contains("not_allowed"), "{text}");
    assert_eq!(
        result["_meta"]["app.getwormhole/error"],
        json!({"code": "not_allowed", "message": "typing is off: turn on \"Let the command line type\" in Settings"})
    );

    let refused: Reply =
        Error::new(ErrorCode::Refused, "not at its prompt").because(Reason::NotAtPrompt).into();
    let app = FakeApp::always(refused);
    let mut client2 = Client::with_app(&app);
    client2.initialize("2025-11-25");
    let result = client2.tool("run", json!({"terminal": 3, "command": "ls"}))["result"].clone();
    assert_eq!(result["_meta"]["app.getwormhole/error"]["reason"], json!("not_at_prompt"));
    assert_eq!(client2.finish(), 0);
    assert_eq!(client.finish(), 0);
}

#[test]
fn wrong_arguments_are_an_error_result_the_model_can_read_and_nothing_reaches_the_app() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let result = client.tool("send_text", json!({"text": "ls"}))["result"].clone();
    assert_eq!(result["isError"], json!(true));
    assert!(result["content"][0]["text"].as_str().expect("text").contains("terminal"));
    assert_eq!(result["_meta"]["app.getwormhole/error"]["code"], json!("bad_request"));
    assert!(app.requests().is_empty());
    assert_eq!(client.finish(), 0);
}

#[test]
fn protocol_mistakes_are_json_rpc_errors() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut client = Client::with_app(&app);
    client.initialize("2025-11-25");
    let unknown_tool = client.call("tools/call", json!({"name": "format_disk", "arguments": {}}));
    assert_eq!(unknown_tool["error"]["code"], json!(-32602));
    assert!(unknown_tool["error"]["message"].as_str().expect("message").contains("format_disk"));
    assert_eq!(client.call("tools/call", json!({}))["error"]["code"], json!(-32602));
    assert_eq!(
        client.call("tools/call", json!({"name": "list_terminals", "arguments": [1]}))["error"]["code"],
        json!(-32602)
    );
    assert_eq!(client.call("resources/list", json!({}))["error"]["code"], json!(-32601));
    assert_eq!(client.call("no/such", json!({}))["error"]["code"], json!(-32601));
    // A method name with a control character in it is not repeated back raw.
    let hostile = client.call("\u{1b}[31mred", json!({}));
    assert!(!hostile.to_string().contains('\u{1b}'));
    assert_eq!(client.call("ping", json!({}))["result"], json!({}));
    assert!(app.requests().is_empty());
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_request_before_initialize_is_told_what_to_send() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut client = Client::with_app(&app);
    let response = client.call("tools/list", json!({}));
    assert_eq!(response["error"]["code"], json!(-32602));
    assert!(response["error"]["message"].as_str().expect("message").contains("initialize"));
    // A ping is always answered.
    assert_eq!(client.call("ping", json!({}))["result"], json!({}));
    assert_eq!(client.finish(), 0);
}

// 2026-07-28

#[test]
fn server_discover_tells_the_versions_capabilities_and_identity() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut client = Client::with_app(&app);
    let result = client.call("server/discover", json!({"_meta": meta()}))["result"].clone();
    assert_eq!(result["resultType"], json!("complete"));
    assert_eq!(
        result["supportedVersions"],
        json!(["2026-07-28", "2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"])
    );
    assert_eq!(result["capabilities"], json!({"tools": {"listChanged": false}}));
    assert_eq!(
        result["_meta"]["io.modelcontextprotocol/serverInfo"],
        json!({"name": "wormhole", "version": version(), "title": "Wormhole"})
    );
    assert!(
        result["instructions"].is_string()
            && result["ttlMs"].is_u64()
            && result["cacheScope"] == json!("private")
    );
    assert_eq!(client.finish(), 0);
}

#[test]
fn stateless_requests_need_no_handshake_and_carry_result_type_and_cache_hints() {
    let app = FakeApp::always(Reply::ok(listing()));
    let mut client = Client::with_app(&app);
    let list = client.call("tools/list", json!({"_meta": meta()}))["result"].clone();
    assert_eq!(list["resultType"], json!("complete"));
    assert_eq!((list["ttlMs"].is_u64(), &list["cacheScope"]), (true, &json!("private")));
    assert_eq!(list["tools"].as_array().expect("tools").len(), 17);
    assert!(list["tools"][0]["outputSchema"].is_object());

    let called = client
        .call("tools/call", json!({"name": "list_terminals", "arguments": {}, "_meta": meta()}))["result"]
        .clone();
    assert_eq!(called["resultType"], json!("complete"));
    assert_eq!(called["isError"], json!(false));
    assert_eq!(called["structuredContent"], listing());
    assert!(called["_meta"]["io.modelcontextprotocol/serverInfo"]["name"] == "wormhole");

    // An error result keeps its own _meta beside the server's.
    let refused = FakeApp::always(Error::new(ErrorCode::NotAllowed, "off").into());
    let mut refused_client = Client::with_app(&refused);
    let result = refused_client.call(
        "tools/call",
        json!({"name": "send_text", "arguments": {"terminal": 3, "text": "x"}, "_meta": meta()}),
    )["result"]
        .clone();
    assert!(
        result["_meta"]["app.getwormhole/error"].is_object()
            && result["_meta"]["io.modelcontextprotocol/serverInfo"].is_object()
    );
    assert_eq!(refused_client.finish(), 0);
    assert_eq!(client.finish(), 0);
}

#[test]
fn an_unsupported_stateless_version_is_told_with_the_ones_we_speak() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut client = Client::with_app(&app);
    let mut other = meta();
    other["io.modelcontextprotocol/protocolVersion"] = json!("2027-01-01");
    let error = client.call("tools/list", json!({"_meta": other}))["error"].clone();
    assert_eq!(error["code"], json!(-32022));
    // Every revision it speaks, newest first, the handshake ones too: a client told to retry with one of them
    // is served (below).
    assert_eq!(
        error["data"],
        json!({"supported": ["2026-07-28", "2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"],
               "requested": "2027-01-01"})
    );
    // A revision with a handshake, named in `_meta` of a request with no handshake before it, is served as
    // that revision (its methods; `server/discover` is not one of them).
    let mut old = meta();
    old["io.modelcontextprotocol/protocolVersion"] = json!("2025-11-25");
    assert_eq!(client.call("server/discover", json!({"_meta": old}))["error"]["code"], json!(-32601));
    let listed = client.call("tools/list", json!({"_meta": old}))["result"].clone();
    assert!(listed["tools"].is_array() && listed.get("resultType").is_none());
    assert_eq!(client.finish(), 0);
}

#[test]
fn a_stateless_request_missing_what_it_must_carry_is_invalid_params() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut client = Client::with_app(&app);
    let no_capabilities = json!({"io.modelcontextprotocol/protocolVersion": "2026-07-28"});
    let response = client.call("tools/list", json!({"_meta": no_capabilities}));
    assert_eq!(response["error"]["code"], json!(-32602));
    let not_a_string = json!({"io.modelcontextprotocol/protocolVersion": 5, "io.modelcontextprotocol/clientCapabilities": {}});
    assert_eq!(client.call("tools/list", json!({"_meta": not_a_string}))["error"]["code"], json!(-32602));
    assert_eq!(client.finish(), 0);
}

#[test]
fn the_methods_the_stateless_revision_removed_are_not_found_there() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut client = Client::with_app(&app);
    for method in ["initialize", "ping", "logging/setLevel"] {
        let response = client.call(method, json!({"_meta": meta(), "protocolVersion": "2025-11-25"}));
        assert_eq!(response["error"]["code"], json!(-32601), "{method}");
    }
    // And discover is not a handshake-era method.
    client.initialize("2025-11-25");
    assert_eq!(client.call("server/discover", json!({}))["error"]["code"], json!(-32601));
    assert_eq!(client.finish(), 0);
}

#[test]
fn both_eras_are_served_on_one_connection() {
    let app = FakeApp::always(Reply::ok(listing()));
    let mut client = Client::with_app(&app);
    client.initialize("2025-06-18");
    let handshake = client.tool("list_terminals", json!({}))["result"].clone();
    assert!(handshake.get("resultType").is_none());
    let stateless =
        client.call("tools/call", json!({"name": "list_terminals", "_meta": meta()}))["result"].clone();
    assert_eq!(stateless["resultType"], json!("complete"));
    assert_eq!(client.finish(), 0);
}

// Batches

#[test]
fn a_batch_is_served_in_2025_03_26_and_refused_elsewhere() {
    let app = FakeApp::always(Reply::ok(listing()));
    let mut client = Client::with_app(&app);
    client.initialize("2025-03-26");
    client.send(&json!([
        {"jsonrpc": "2.0", "id": 1, "method": "ping"},
        {"jsonrpc": "2.0", "method": "notifications/initialized"},
        {"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "list_terminals"}},
        {"jsonrpc": "2.0", "id": 3, "method": "nope"},
        {"nope": true}
    ]));
    let answers = client.read();
    let answers = answers.as_array().expect("one array");
    assert_eq!(answers.len(), 4, "{answers:?}");
    assert_eq!(answers[0], json!({"jsonrpc": "2.0", "id": 1, "result": {}}));
    assert_eq!(answers[1]["result"]["isError"], json!(false));
    assert_eq!(answers[2]["error"]["code"], json!(-32601));
    assert_eq!(answers[3]["error"]["code"], json!(-32600));
    assert_eq!(client.finish(), 0);

    for requested in ["2025-06-18", "2025-11-25", "2024-11-05"] {
        let mut client = Client::with_app(&app);
        client.initialize(requested);
        client.send(&json!([{"jsonrpc": "2.0", "id": 1, "method": "ping"}]));
        let answer = client.read();
        assert_eq!(answer["id"], json!(null), "{requested}: {answer}");
        assert_eq!(answer["error"]["code"], json!(-32600));
        assert!(answer["error"]["message"].as_str().expect("message").contains("2025-03-26"));
        // The connection goes on.
        assert_eq!(client.call("ping", json!({}))["result"], json!({}));
        assert_eq!(client.finish(), 0);
    }

    // Before any handshake, and in the stateless flow, there is none either.
    let mut fresh = Client::with_app(&app);
    fresh.send(&json!([{"jsonrpc": "2.0", "id": 1, "method": "ping"}]));
    assert_eq!(fresh.read()["error"]["code"], json!(-32600));
    assert_eq!(fresh.finish(), 0);
}

#[test]
fn an_app_that_is_not_running_is_an_error_result_that_says_so() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut endpoint = app.endpoint();
    endpoint.caller.socket = endpoint.caller.socket.with_file_name("nobody-listens.sock");
    let mut client = Client::start(endpoint);
    client.initialize("2025-11-25");
    let result = client.tool("list_terminals", json!({}))["result"].clone();
    assert_eq!(result["isError"], json!(true));
    assert!(result["content"][0]["text"].as_str().expect("text").contains("isn't running"));
    assert_eq!(result["_meta"]["app.getwormhole/error"]["code"], json!("unavailable"));
    assert_eq!(client.finish(), 0);
}

// The Automation flag

#[test]
fn while_the_flag_is_off_only_the_tools_that_work_without_it_are_offered_and_the_instructions_say_so() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut endpoint = app.endpoint();
    endpoint.automation = || false;
    let mut client = Client::start(endpoint);
    let init = client.initialize("2025-11-25");
    let instructions = init["instructions"].as_str().expect("instructions");
    assert!(instructions.contains("Automation feature flag is off"), "{instructions}");
    let listed = client.call("tools/list", json!({}))["result"]["tools"].clone();
    assert_eq!(
        names(&listed),
        ["list_terminals", "terminal_status", "notify", "send_text", "new_terminal"],
        "the five the command line always had"
    );
    // The stateless flow says it too, with a cache hint that lets a person turn the flag on and see the rest.
    let discovered = client.call("server/discover", json!({"_meta": meta()}))["result"].clone();
    assert!(discovered["instructions"].as_str().expect("text").contains("flag is off"));
    assert_eq!(discovered["ttlMs"], json!(60_000));
    // A tool that is not offered is still the app's to answer, if a client calls it anyway.
    let result = client.tool("list_blocks", json!({"terminal": 3}))["result"].clone();
    assert_eq!(result["isError"], json!(false));
    assert_eq!(app.requests().len(), 1);
    assert_eq!(client.finish(), 0);

    // With the flag on, all of them, and no word about it.
    let mut client = Client::with_app(&app);
    let init = client.initialize("2025-11-25");
    assert!(!init["instructions"].as_str().expect("instructions").contains("flag is off"));
    assert_eq!(client.call("tools/list", json!({}))["result"]["tools"].as_array().expect("tools").len(), 17);
    assert_eq!(client.finish(), 0);
}

#[test]
fn initialize_is_not_part_of_a_batch() {
    let app = FakeApp::always(Reply::ok(json!({})));
    let mut client = Client::with_app(&app);
    client.initialize("2025-03-26");
    client.send(&json!([
        {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-03-26"}},
        {"jsonrpc": "2.0", "id": 2, "method": "ping"}
    ]));
    let answers = client.read();
    let answers = answers.as_array().expect("array");
    assert_eq!((answers[0]["id"].clone(), answers[0]["error"]["code"].clone()), (json!(1), json!(-32600)));
    assert_eq!(answers[1]["result"], json!({}));
    assert_eq!(client.finish(), 0);
}
