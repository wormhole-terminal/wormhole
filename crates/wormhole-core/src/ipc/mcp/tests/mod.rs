//! The server end to end: JSON-RPC messages in at one end, a fake app on a real socket at the other.
//! The harness is shared by the test files beside it.

mod calls;
mod events;
mod hostile;
mod protocol;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::Shutdown;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::exec::Endpoint;
use super::serve;
use crate::ipc::Message;
use crate::ipc::control::printable::json_line;
use crate::ipc::control::{Reply, Request};
use crate::ipc::exchange::Caller;
use crate::platform::socket::{Listener, Stream};

/// How long a test waits for anything before it calls the server hung.
const PATIENCE: Duration = Duration::from_secs(10);

/// What the fake app does with a request.
pub(super) enum Behavior {
    /// One line back.
    Reply(Reply),
    /// Say nothing and keep the connection until the other end goes (a `wait` in progress).
    Hang,
    /// The reply, then these lines, until the receiver is dropped or the other end goes.
    Stream { reply: Reply, lines: Receiver<String> },
}

/// A stand-in for the app on a socket of its own: it records the requests and answers as told.
pub(super) struct FakeApp {
    socket: PathBuf,
    _dir: tempfile::TempDir,
    seen: Arc<Mutex<Vec<Request>>>,
    /// How many connections the other end closed on it.
    hung_up: Arc<AtomicUsize>,
}

impl FakeApp {
    pub(super) fn start(handler: impl Fn(&Request) -> Behavior + Send + Sync + 'static) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let socket = dir.path().join("fake.sock");
        let listener = Listener::bind(&socket).expect("bind");
        let seen: Arc<Mutex<Vec<Request>>> = Arc::default();
        let hung_up: Arc<AtomicUsize> = Arc::default();
        let handler = Arc::new(handler);
        let (seen_by, hung_by) = (Arc::clone(&seen), Arc::clone(&hung_up));
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (handler, seen, hung_up) =
                    (Arc::clone(&handler), Arc::clone(&seen_by), Arc::clone(&hung_by));
                std::thread::spawn(move || serve_one(stream, &*handler, &seen, &hung_up));
            }
        });
        Self { socket, _dir: dir, seen, hung_up }
    }

    /// An app that answers every request with the same reply.
    pub(super) fn always(reply: Reply) -> Self {
        Self::start(move |_| Behavior::Reply(reply.clone()))
    }

    pub(super) fn endpoint(&self) -> Endpoint {
        Endpoint {
            caller: Caller { socket: self.socket.clone(), from: Some("7".into()) },
            cwd: PathBuf::from("/work"),
            home: Some(PathBuf::from("/home/me")),
        }
    }

    /// The requests received so far, as the wire has them.
    pub(super) fn requests(&self) -> Vec<Value> {
        self.seen.lock().expect("lock").iter().map(|r| serde_json::to_value(r).expect("serializes")).collect()
    }

    pub(super) fn hung_up(&self) -> usize {
        self.hung_up.load(Ordering::SeqCst)
    }

    /// Wait until `count` requests have arrived.
    pub(super) fn wait_for_requests(&self, count: usize) {
        until(|| self.requests().len() >= count, "the app to receive the requests");
    }
}

fn serve_one(
    mut stream: Stream,
    handler: &(impl Fn(&Request) -> Behavior + ?Sized),
    seen: &Mutex<Vec<Request>>,
    hung_up: &AtomicUsize,
) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let Ok(Message::Control(request)) = serde_json::from_str(&line) else { return };
    let behavior = handler(&request);
    seen.lock().expect("lock").push(request);
    let write = |stream: &mut Stream, text: &str| stream.write_all(format!("{text}\n").as_bytes()).is_ok();
    // As the real server tells (`stream::client_gone`): an empty write to a client that closed fails.
    let gone = |stream: &mut Stream| {
        matches!(stream.write(&[]), Err(err) if matches!(err.kind(),
            std::io::ErrorKind::BrokenPipe | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::NotConnected))
    };
    match behavior {
        Behavior::Reply(reply) => {
            write(&mut stream, &json_line(&reply));
        }
        Behavior::Hang => loop {
            std::thread::sleep(Duration::from_millis(10));
            if gone(&mut stream) {
                hung_up.fetch_add(1, Ordering::SeqCst);
                return;
            }
        },
        Behavior::Stream { reply, lines } => {
            if !write(&mut stream, &json_line(&reply)) {
                return;
            }
            loop {
                match lines.recv_timeout(Duration::from_millis(50)) {
                    Ok(text) => {
                        if !write(&mut stream, &text) {
                            hung_up.fetch_add(1, Ordering::SeqCst);
                            return;
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if gone(&mut stream) {
                            hung_up.fetch_add(1, Ordering::SeqCst);
                            return;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        }
    }
}

/// Poll `condition` until it holds (a test that never sees it fails rather than hangs).
pub(super) fn until(mut condition: impl FnMut() -> bool, what: &str) {
    let deadline = Instant::now() + PATIENCE;
    while !condition() {
        assert!(Instant::now() < deadline, "gave up waiting for {what}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

/// A client of the server, over a pair of sockets standing in for its stdin and stdout.
pub(super) struct Client {
    to_server: Stream,
    from_server: BufReader<Stream>,
    server: Option<JoinHandle<i32>>,
    next_id: i64,
}

impl Client {
    pub(super) fn start(endpoint: Endpoint) -> Self {
        let (to_server, server_input) = Stream::pair().expect("pair");
        let (server_output, from_server) = Stream::pair().expect("pair");
        from_server.set_read_timeout(Some(PATIENCE)).expect("timeout");
        let server = std::thread::spawn(move || serve(BufReader::new(server_input), server_output, endpoint));
        Self { to_server, from_server: BufReader::new(from_server), server: Some(server), next_id: 0 }
    }

    /// A client of a fake app that answers each request as `reply`.
    pub(super) fn with_app(app: &FakeApp) -> Self {
        Self::start(app.endpoint())
    }

    pub(super) fn send(&mut self, message: &Value) {
        self.send_raw(format!("{message}\n").as_bytes());
    }

    pub(super) fn send_raw(&mut self, bytes: &[u8]) {
        self.to_server.write_all(bytes).expect("the server reads");
    }

    /// The next line the server wrote, as it is on the wire.
    pub(super) fn read_raw(&mut self) -> String {
        let mut line = String::new();
        match self.from_server.read_line(&mut line) {
            Ok(0) => panic!("the server closed its output"),
            Ok(_) => line,
            Err(err) => panic!("no word from the server: {err}"),
        }
    }

    pub(super) fn read(&mut self) -> Value {
        let line = self.read_raw();
        serde_json::from_str(&line)
            .unwrap_or_else(|err| panic!("the server wrote something that is not JSON ({err}): {line}"))
    }

    /// The next message if one comes within `within`.
    pub(super) fn read_within(&mut self, within: Duration) -> Option<Value> {
        self.from_server.get_ref().set_read_timeout(Some(within)).expect("timeout");
        let mut line = String::new();
        let read = self.from_server.read_line(&mut line);
        self.from_server.get_ref().set_read_timeout(Some(PATIENCE)).expect("timeout");
        match read {
            Ok(n) if n > 0 => Some(serde_json::from_str(&line).expect("JSON")),
            _ => None,
        }
    }

    /// Send a request and return the response to it (the messages that are not it are not expected here).
    pub(super) fn call(&mut self, method: &str, params: Value) -> Value {
        let id = self.request(method, params);
        let response = self.read();
        assert_eq!(response["id"], json!(id), "{response}");
        response
    }

    /// Send a request without waiting; returns its id.
    pub(super) fn request(&mut self, method: &str, params: Value) -> i64 {
        self.next_id += 1;
        let id = self.next_id;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        id
    }

    pub(super) fn notify(&mut self, method: &str, params: Value) {
        self.send(&json!({"jsonrpc": "2.0", "method": method, "params": params}));
    }

    /// `initialize` as `version`; returns the result.
    pub(super) fn initialize(&mut self, version: &str) -> Value {
        let response = self.call(
            "initialize",
            json!({"protocolVersion": version, "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}),
        );
        self.notify("notifications/initialized", json!({}));
        response["result"].clone()
    }

    /// `tools/call` as the `2025-11-25` flow has it (after [`Client::initialize`]); the response.
    pub(super) fn tool(&mut self, name: &str, arguments: Value) -> Value {
        self.call("tools/call", json!({"name": name, "arguments": arguments}))
    }

    /// Close the input, as a client does when it is done; the server's exit code.
    pub(super) fn finish(mut self) -> i32 {
        let _ = self.to_server.shutdown(Shutdown::Write);
        let code = self.server.take().expect("running").join().expect("the server did not panic");
        // Nothing else was written after the last message the test read.
        let mut rest = String::new();
        let _ = self.from_server.read_to_string(&mut rest);
        assert_eq!(rest, "", "nothing more was written");
        code
    }
}

/// The `_meta` a `2026-07-28` request carries.
pub(super) fn meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": {"name": "test", "version": "1"}
    })
}

/// A listing of one terminal, as `list` answers.
pub(super) fn listing() -> Value {
    json!({"windows": [{"id": 1, "front": true, "terminals": [{
        "id": 3, "name": "api", "status": "working", "pinned": false, "selected": true,
        "tabs": [{"id": 3, "name": "api", "status": "working", "shell": "zsh", "since_seconds": 4,
                  "selected": true, "panes": [{"id": 3}]}]
    }]}]})
}
