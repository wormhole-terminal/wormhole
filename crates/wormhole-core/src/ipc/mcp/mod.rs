//! `wh mcp`: the control protocol served to coding agents over MCP (the Model Context Protocol), on
//! stdin and stdout, one JSON-RPC message per line.
//!
//! It is a client of the same socket `wh` talks to and has no policy of its own. Each tool builds the
//! request the command line would build and sends it with the same exchange; the switches
//! (`control_write`, `control_read`), the budgets, the refusals, the automation flag and the checks on who
//! is calling are all the app's. The caller is this process: it runs in the agent's terminal, so the tab it
//! names (`WORMHOLE_TAB_ID`) and its place in the process tree are the agent's, and it can read its own
//! terminal without `control_read` and can never type into it, as with `wh`.
//!
//! - `rpc`: JSON-RPC 2.0 over lines: parsing, the line cap, the writer;
//! - `version`: the protocol revisions (2026-07-28 and the four before it) and what each changes;
//! - `server`: methods, the revision a request is served as, `tools/call` and its progress;
//! - `calls`: the calls in flight, at most four, and how a cancellation finds one;
//! - `tools`: the tool table: descriptions, schemas, annotations and the arguments each reads;
//! - `exec`: one request to the app, as `wh` makes it; `result`: a reply as a tool result;
//! - `feed`: the events tool's buffer of the app's stream.
//!
//! stdout carries protocol and nothing else; diagnostics go to stderr. Tier: with `exchange`, which it
//! uses; nothing above `ipc` is named.

mod calls;
mod exec;
mod feed;
mod result;
mod rpc;
mod server;
#[cfg(test)]
mod tests;
mod tools;
mod version;

use std::io::{self, BufRead, Write};
use std::sync::Arc;
use std::time::Duration;

use exec::Endpoint;
use rpc::{LINE_MAX, Line, Out, RpcError, code};
use server::Server;

use super::control::print_err;

/// How long a call that is running when the client closes its input is given to answer.
const GRACE: Duration = Duration::from_secs(3);

/// `wh mcp`: serve stdin and stdout until the client closes its input. The exit code.
pub(super) fn run(args: &[String]) -> i32 {
    if let Some(arg) = args.first() {
        let wh = super::control::COMMAND_NAME;
        let usage = format!(
            "{wh} mcp serves Wormhole's tools to a coding agent over MCP on stdin and stdout; it takes no arguments.\n\
             Add it to an agent, e.g. `claude mcp add wormhole -- {wh} mcp`.\n"
        );
        if matches!(arg.as_str(), "-h" | "--help" | "help") {
            super::control::print(&usage);
            return 0;
        }
        print_err(&format!("Wormhole: mcp takes no arguments, not {arg:?}\n\n{usage}"));
        return 2;
    }
    serve(io::stdin().lock(), io::stdout(), Endpoint::from_env())
}

/// Serve `input` and `output` as an MCP server to `endpoint`'s app, until `input` ends.
fn serve(mut input: impl BufRead, output: impl Write + Send + 'static, endpoint: Endpoint) -> i32 {
    let server = Arc::new(Server::new(endpoint, Out::new(output)));
    loop {
        match rpc::read_line(&mut input, LINE_MAX) {
            Ok(Line::Text(bytes)) if bytes.iter().all(u8::is_ascii_whitespace) => {}
            Ok(Line::Text(bytes)) => server.serve_line(&bytes),
            Ok(Line::TooLong) => {
                let error =
                    RpcError::new(code::INVALID_REQUEST, format!("a message is at most {LINE_MAX} bytes"));
                server.out.send(&rpc::error_response(&serde_json::Value::Null, &error));
            }
            Ok(Line::End) => break,
            Err(err) => {
                print_err(&format!("wh mcp: stdin: {err}\n"));
                break;
            }
        }
        if server.out.is_broken() {
            print_err("wh mcp: stdout is closed; exiting\n");
            break;
        }
    }
    // The client is done, or gone: give what runs a moment to answer, then end it.
    if server.out.is_broken() {
        server.calls.cancel_all();
    }
    if !server.calls.wait_idle(GRACE) {
        server.calls.cancel_all();
    }
    0
}
