//! JSON-RPC 2.0 as MCP's stdio transport carries it: one message (or, where the protocol revision allows
//! it, one batch) per line. Everything on the other end is hostile input: a line is read within a cap,
//! and what does not parse is answered with the error the spec names and never panics.

use std::io::{self, BufRead, Write};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::ipc::control::printable::json_line;

/// The most one incoming line may have. Tool arguments are small (the biggest, a command or typed text,
/// is 16 KiB); a megabyte is room for any honest client.
pub(super) const LINE_MAX: usize = 1 << 20;

/// JSON-RPC's error codes (and MCP's `-32022`, which `2026-07-28` allots to an unsupported version).
pub(super) mod code {
    pub(in crate::ipc::mcp) const PARSE_ERROR: i64 = -32700;
    pub(in crate::ipc::mcp) const INVALID_REQUEST: i64 = -32600;
    pub(in crate::ipc::mcp) const METHOD_NOT_FOUND: i64 = -32601;
    pub(in crate::ipc::mcp) const INVALID_PARAMS: i64 = -32602;
    pub(in crate::ipc::mcp) const INTERNAL_ERROR: i64 = -32603;
    pub(in crate::ipc::mcp) const UNSUPPORTED_VERSION: i64 = -32022;
}

/// A JSON-RPC error object.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct RpcError {
    pub(super) code: i64,
    pub(super) message: String,
    pub(super) data: Option<Value>,
}

impl RpcError {
    pub(super) fn new(code: i64, message: impl Into<String>) -> Self {
        Self { code, message: message.into(), data: None }
    }

    pub(super) fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    pub(super) fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(code::INVALID_PARAMS, message)
    }

    fn to_value(&self) -> Value {
        let mut error = json!({"code": self.code, "message": self.message});
        if let Some(data) = &self.data {
            error["data"] = data.clone();
        }
        error
    }
}

/// One message from the client.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Message {
    /// Wants an answer with the same `id` (a string or an integer, never null).
    Request { id: Value, method: String, params: Value },
    /// Wants none.
    Notification { method: String, params: Value },
    /// An answer to a request this server never sends: nothing to do with it.
    Response,
}

/// A message that cannot be served, and the answer it gets: the `id` it came with, or null when that could
/// not be read either.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Rejected {
    pub(super) id: Value,
    pub(super) error: RpcError,
}

impl Rejected {
    fn invalid(id: Value, message: impl Into<String>) -> Box<Self> {
        Box::new(Self { id, error: RpcError::new(code::INVALID_REQUEST, message) })
    }

    pub(super) fn answer(&self) -> Value {
        error_response(&self.id, &self.error)
    }
}

/// What one line holds.
#[derive(Debug, PartialEq)]
pub(super) enum Frame {
    Single(Result<Message, Box<Rejected>>),
    /// A JSON-RPC batch: whether it may be served depends on the protocol revision.
    Batch(Vec<Result<Message, Box<Rejected>>>),
    /// Not even a message.
    Rejected(Box<Rejected>),
}

/// Read one line of input.
pub(super) fn parse(line: &[u8]) -> Frame {
    let value: Value = match serde_json::from_slice(line) {
        Ok(value) => value,
        Err(err) => {
            return Frame::Rejected(Box::new(Rejected {
                id: Value::Null,
                error: RpcError::new(code::PARSE_ERROR, format!("not JSON: {err}")),
            }));
        }
    };
    match value {
        Value::Array(items) if items.is_empty() => {
            Frame::Rejected(Rejected::invalid(Value::Null, "an empty batch"))
        }
        Value::Array(items) => Frame::Batch(items.into_iter().map(message).collect()),
        other => Frame::Single(message(other)),
    }
}

/// An `id` MCP allows: a string or an integer.
fn valid_id(id: &Value) -> bool {
    id.is_string() || id.is_i64() || id.is_u64()
}

fn message(value: Value) -> Result<Message, Box<Rejected>> {
    let Value::Object(mut object) = value else {
        return Err(Rejected::invalid(Value::Null, "a message is a JSON object"));
    };
    let id = object.remove("id");
    // What can be answered to an id that is fit to be answered to.
    let reply_id = id.clone().filter(valid_id).unwrap_or(Value::Null);
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(Rejected::invalid(reply_id, "`jsonrpc` must be \"2.0\""));
    }
    let Some(method) = object.remove("method") else {
        let is_response = object.contains_key("result") || object.contains_key("error");
        return if is_response && id.as_ref().is_some_and(valid_id) {
            Ok(Message::Response)
        } else {
            Err(Rejected::invalid(reply_id, "neither a request, a notification nor a response"))
        };
    };
    let Value::String(method) = method else {
        return Err(Rejected::invalid(reply_id, "`method` must be a string"));
    };
    let params = object.remove("params");
    match id {
        None => {
            // A notification gets no answer, so a params that is not an object is not worth one either.
            let params = match params {
                Some(Value::Object(params)) => Value::Object(params),
                _ => Value::Object(Map::new()),
            };
            Ok(Message::Notification { method, params })
        }
        Some(id) if valid_id(&id) => match params {
            None => Ok(Message::Request { id, method, params: Value::Object(Map::new()) }),
            Some(params @ Value::Object(_)) => Ok(Message::Request { id, method, params }),
            Some(_) => {
                Err(Box::new(Rejected { id, error: RpcError::invalid_params("`params` must be an object") }))
            }
        },
        Some(_) => Err(Rejected::invalid(Value::Null, "a request's `id` is a string or an integer")),
    }
}

/// The response to a request that worked.
pub(super) fn result_response(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// The response to a request that did not.
pub(super) fn error_response(id: &Value, error: &RpcError) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": error.to_value()})
}

pub(super) fn notification(method: &str, params: Value) -> Value {
    json!({"jsonrpc": "2.0", "method": method, "params": params})
}

/// What reading a line came to.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Line {
    /// A line within the cap, without its newline.
    Text(Vec<u8>),
    /// A line over the cap, skipped to its end.
    TooLong,
    End,
}

/// The next line of `input`, within [`LINE_MAX`]: the rest of a longer one is thrown away unread into memory,
/// so a client cannot make this process hold what it sends.
pub(super) fn read_line(input: &mut impl BufRead, max: usize) -> io::Result<Line> {
    let mut line = Vec::new();
    let mut taken = Vec::new();
    loop {
        // One byte more than the cap says whether the line went over.
        let room = (max + 1).saturating_sub(line.len()) as u64;
        taken.clear();
        let read = io::Read::take(&mut *input, room).read_until(b'\n', &mut taken)?;
        if read == 0 {
            return Ok(if line.is_empty() { Line::End } else { Line::Text(line) });
        }
        let ended = taken.last() == Some(&b'\n');
        line.extend_from_slice(&taken);
        if ended {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
        }
        if line.len() > max {
            if !ended {
                skip_line(input)?;
            }
            return Ok(Line::TooLong);
        }
        if ended {
            return Ok(Line::Text(line));
        }
    }
}

fn skip_line(input: &mut impl BufRead) -> io::Result<()> {
    loop {
        let (used, found) = {
            let buffer = input.fill_buf()?;
            if buffer.is_empty() {
                return Ok(());
            }
            match buffer.iter().position(|b| *b == b'\n') {
                Some(at) => (at + 1, true),
                None => (buffer.len(), false),
            }
        };
        input.consume(used);
        if found {
            return Ok(());
        }
    }
}

/// Where the messages go: stdout, one line each, never anything else. Writes are whole lines under a lock,
/// so the calls running on threads of their own cannot interleave them.
pub(super) struct Out {
    sink: Mutex<Box<dyn Write + Send>>,
    broken: AtomicBool,
}

impl Out {
    pub(super) fn new(sink: impl Write + Send + 'static) -> Self {
        Self { sink: Mutex::new(Box::new(sink)), broken: AtomicBool::new(false) }
    }

    /// Write `message` as a line (`false` when the client has gone away). Characters that could pass for
    /// controls in a terminal are escaped as the control socket's JSON lines do.
    pub(super) fn send(&self, message: &impl Serialize) -> bool {
        let mut line = json_line(message);
        line.push('\n');
        let mut sink = self.sink.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let wrote = sink.write_all(line.as_bytes()).and_then(|()| sink.flush()).is_ok();
        if !wrote {
            self.broken.store(true, Ordering::SeqCst);
        }
        wrote
    }

    /// Whether a write failed: nobody is listening any more.
    pub(super) fn is_broken(&self) -> bool {
        self.broken.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use serde_json::json;

    use super::{Frame, Line, Message, code, parse, read_line};

    fn single(line: &str) -> Result<Message, Box<super::Rejected>> {
        match parse(line.as_bytes()) {
            Frame::Single(message) => message,
            other => panic!("not a single message: {other:?}"),
        }
    }

    #[test]
    fn requests_notifications_and_responses_are_told_apart() {
        let request = single(r#"{"jsonrpc":"2.0","id":7,"method":"tools/list"}"#).expect("a request");
        assert_eq!(
            request,
            Message::Request { id: json!(7), method: "tools/list".into(), params: json!({}) }
        );
        let string_id = single(r#"{"jsonrpc":"2.0","id":"a-1","method":"ping","params":{"x":1}}"#);
        assert!(matches!(string_id, Ok(Message::Request { ref id, .. }) if id == "a-1"));
        let note =
            single(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).expect("a notification");
        assert_eq!(
            note,
            Message::Notification { method: "notifications/initialized".into(), params: json!({}) }
        );
        assert_eq!(single(r#"{"jsonrpc":"2.0","id":3,"result":{}}"#), Ok(Message::Response));
    }

    #[test]
    fn what_is_not_a_message_is_answered_with_the_errors_the_spec_names() {
        let Frame::Rejected(bad) = parse(b"{not json") else { panic!("parsed") };
        assert_eq!((bad.error.code, bad.id), (code::PARSE_ERROR, json!(null)));
        let cases = [
            (r#"[]"#, code::INVALID_REQUEST),
            (r#"3"#, code::INVALID_REQUEST),
            (r#"{"id":1,"method":"x"}"#, code::INVALID_REQUEST),
            (r#"{"jsonrpc":"1.0","id":1,"method":"x"}"#, code::INVALID_REQUEST),
            (r#"{"jsonrpc":"2.0","id":1}"#, code::INVALID_REQUEST),
            (r#"{"jsonrpc":"2.0","id":null,"method":"x"}"#, code::INVALID_REQUEST),
            (r#"{"jsonrpc":"2.0","id":1.5,"method":"x"}"#, code::INVALID_REQUEST),
            (r#"{"jsonrpc":"2.0","id":{},"method":"x"}"#, code::INVALID_REQUEST),
            (r#"{"jsonrpc":"2.0","id":1,"method":5}"#, code::INVALID_REQUEST),
        ];
        for (line, expected) in cases {
            let rejected = match parse(line.as_bytes()) {
                Frame::Single(Err(rejected)) | Frame::Rejected(rejected) => rejected,
                other => panic!("{line} read as {other:?}"),
            };
            assert_eq!(rejected.error.code, expected, "{line}");
        }
        // The id is given back when it is fit to be: the client can tell which request failed.
        let params =
            single(r#"{"jsonrpc":"2.0","id":9,"method":"tools/call","params":[1]}"#).expect_err("bad");
        assert_eq!((params.error.code, params.id), (code::INVALID_PARAMS, json!(9)));
        let wrong = single(r#"{"jsonrpc":"1.0","id":"q","method":"x"}"#).expect_err("bad");
        assert_eq!(wrong.id, json!("q"));
    }

    #[test]
    fn a_batch_is_read_element_by_element() {
        let Frame::Batch(items) = parse(
            br#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"nope":1},{"jsonrpc":"2.0","method":"notifications/initialized"}]"#,
        ) else {
            panic!("not a batch")
        };
        assert_eq!(items.len(), 3);
        assert!(items[0].is_ok() && items[1].is_err() && items[2].is_ok());
    }

    #[test]
    fn a_long_line_is_skipped_to_its_end_and_the_next_one_still_reads() {
        let mut input = Vec::new();
        input.extend_from_slice(&[b'x'; 100]);
        input.extend_from_slice(b"\nshort\r\n");
        let mut cursor = Cursor::new(input);
        assert_eq!(read_line(&mut cursor, 10).expect("reads"), Line::TooLong);
        assert_eq!(read_line(&mut cursor, 10).expect("reads"), Line::Text(b"short".to_vec()));
        assert_eq!(read_line(&mut cursor, 10).expect("reads"), Line::End);
    }

    #[test]
    fn a_line_exactly_at_the_cap_is_kept_and_one_byte_over_is_not() {
        let mut at = Cursor::new(b"0123456789\n0123456789A\nlast".to_vec());
        assert_eq!(read_line(&mut at, 10).expect("reads"), Line::Text(b"0123456789".to_vec()));
        assert_eq!(read_line(&mut at, 10).expect("reads"), Line::TooLong);
        // A last line without its newline is a line too.
        assert_eq!(read_line(&mut at, 10).expect("reads"), Line::Text(b"last".to_vec()));
    }

    #[test]
    fn a_long_line_without_any_newline_is_not_held_in_memory() {
        let mut huge = Cursor::new(vec![b'y'; 5 << 20]);
        assert_eq!(read_line(&mut huge, 1 << 10).expect("reads"), Line::TooLong);
        assert_eq!(read_line(&mut huge, 1 << 10).expect("reads"), Line::End);
    }
}
