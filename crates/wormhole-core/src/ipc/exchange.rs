//! The command line's round trip: `Wormhole <verb> …` sends one request over the control socket and
//! prints the app's reply (`run`), and the same exchange serves the welcome's terminal (`exchange`). The
//! text of the verbs, their parsing and the printing of a reply are `control`'s; this is the half that
//! talks, and the only one that names [`Message`], so `control` (the protocol's types, which `Message`
//! carries) stays below it.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::Shutdown;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::Message;
use super::control::Parsed;
use super::control::event::{HEARTBEAT, StreamLine, Subscribed};
use super::control::events;
use super::control::printable::json_line;
use super::control::sessions;
use super::control::{
    Action, PROTOCOL_VERSION, Ran, Reason, Reply, Request, SEND_TEXT_MAX_BYTES, help_with, json_env, parse,
    print, print_err, render_with, run_parts, show_automation,
};
use crate::platform::paths;
use crate::platform::socket::Stream;

/// Why no reply came.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Exchange {
    /// Nothing listens on the socket.
    NotRunning,
    /// The app took the request and said nothing: an older build that doesn't know control
    /// requests, or one too busy for [`REPLY_WAIT`].
    NoAnswer,
    /// The request could not be written, or the answer could not be read.
    Broken(String),
}

/// How much of an answer is read: twice what the app sends at most (`REPLY_MAX_BYTES`), for the
/// escapes a JSON writer adds.
const REPLY_LIMIT: u64 = 2 << 20;

/// How long to wait for the app's answer: it answers on its UI thread, in a moment, unless it
/// is starting up or an older build is running that never will.
const REPLY_WAIT: Duration = Duration::from_secs(10);

impl Exchange {
    /// What went wrong, in a sentence.
    pub(crate) fn words(&self) -> String {
        match self {
            Exchange::NotRunning => "Wormhole isn't running".to_string(),
            Exchange::NoAnswer => "the running Wormhole didn't answer".to_string(),
            Exchange::Broken(why) => why.clone(),
        }
    }
}

/// Send `request` to the app at `socket` and read its reply, waiting at most `wait` for it.
pub(crate) fn exchange(socket: &Path, request: &Request, wait: Duration) -> Result<Reply, Exchange> {
    let mut stream = Stream::connect(socket).map_err(|_| Exchange::NotRunning)?;
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_read_timeout(Some(wait));
    let line = serde_json::to_string(&Message::Control(request.clone()))
        .map_err(|e| Exchange::Broken(e.to_string()))?;
    writeln!(stream, "{line}").map_err(|e| Exchange::Broken(e.to_string()))?;
    // The app reads lines until the connection ends on our side; the answer comes back on it.
    let _ = stream.shutdown(Shutdown::Write);
    let mut answer = String::new();
    match BufReader::new(stream).take(REPLY_LIMIT).read_line(&mut answer) {
        Ok(0) => Err(Exchange::NoAnswer),
        Ok(_) => {
            serde_json::from_str(&answer).map_err(|e| Exchange::Broken(format!("unreadable reply: {e}")))
        }
        Err(err) if matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {
            Err(Exchange::NoAnswer)
        }
        Err(err) => Err(Exchange::Broken(err.to_string())),
    }
}

/// How a stream ended.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum StreamEnd {
    /// A callback said stop, with this exit code.
    Stopped(i32),
    /// The connection ended without the app saying why: it quit or died between two lines.
    Hangup,
}

/// A stream's exchange: send `request`, hand its reply line to `on_reply`, then each line after it to
/// `on_line`, until one of them breaks with an exit code or the connection ends. The reply
/// is waited for as long as any reply; after it, a stream that is silent for three heartbeats is
/// taken to have lost the app.
pub(crate) fn exchange_stream(
    socket: &Path,
    request: &Request,
    on_reply: impl FnOnce(&str, Reply) -> ControlFlow<i32>,
    mut on_line: impl FnMut(&str) -> ControlFlow<i32>,
) -> Result<StreamEnd, Exchange> {
    let mut stream = Stream::connect(socket).map_err(|_| Exchange::NotRunning)?;
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_read_timeout(Some(REPLY_WAIT));
    let line = serde_json::to_string(&Message::Control(request.clone()))
        .map_err(|e| Exchange::Broken(e.to_string()))?;
    writeln!(stream, "{line}").map_err(|e| Exchange::Broken(e.to_string()))?;
    // The app doesn't read after the request: shutting our side says nothing it needs.
    let _ = stream.shutdown(Shutdown::Write);
    let mut reader = BufReader::new(stream);
    let text = read_stream_line(&mut reader, true)?.ok_or(Exchange::NoAnswer)?;
    let reply: Reply =
        serde_json::from_str(&text).map_err(|e| Exchange::Broken(format!("unreadable reply: {e}")))?;
    // After a good reply the lines come at their own pace.
    let _ = reader.get_ref().set_read_timeout(Some(3 * HEARTBEAT));
    if let ControlFlow::Break(code) = on_reply(&text, reply) {
        return Ok(StreamEnd::Stopped(code));
    }
    while let Some(text) = read_stream_line(&mut reader, false)? {
        if let ControlFlow::Break(code) = on_line(&text) {
            return Ok(StreamEnd::Stopped(code));
        }
    }
    Ok(StreamEnd::Hangup)
}

/// The next line of a stream without its newline, or `None` at its end. One line is at most
/// [`LINE_MAX`] bytes, whatever it is made of: a stream is a long time to be trusting.
fn read_stream_line(reader: &mut BufReader<Stream>, reply: bool) -> Result<Option<String>, Exchange> {
    let mut line = String::new();
    match reader.by_ref().take(LINE_MAX).read_line(&mut line) {
        Ok(0) => Ok(None),
        Ok(_) if !line.ends_with('\n') => Err(Exchange::Broken("a line was too long".into())),
        Ok(_) => Ok(Some(line.trim_end().to_string())),
        Err(err) if matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {
            Err(if reply {
                Exchange::NoAnswer
            } else {
                Exchange::Broken(format!("no word from Wormhole for {} s", (3 * HEARTBEAT).as_secs()))
            })
        }
        Err(err) => Err(Exchange::Broken(err.to_string())),
    }
}

/// How many bytes one line of a stream may have.
const LINE_MAX: u64 = 2 << 20;

/// `Wormhole <verb> …`: do the round trip and print the answer. Returns the exit code.
pub fn run(args: &[String]) -> i32 {
    // Session hosts are not the app's: this works from their files, with the app closed too.
    if args.first().map(String::as_str) == Some("sessions") {
        return sessions::run(&args[1..]);
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let home = paths::home();
    let usage = || help_with(show_automation());
    let cli = match parse(args, &cwd, home.as_deref()) {
        Ok(Parsed::Help) => {
            print(&usage());
            return 0;
        }
        Ok(Parsed::Run(cli)) => cli,
        Err(mistake) => {
            print_err(&format!("Wormhole: {mistake}\n\n{}", usage()));
            return 2;
        }
    };
    let json = cli.wants_json(json_env().as_deref());
    let numbers = cli.numbers;
    let mut action = cli.action;
    if cli.stdin
        && let Action::SendText { text, .. } = &mut action
    {
        match read_stdin_text() {
            Ok(read) => *text = read,
            Err(mistake) => {
                print_err(&format!("Wormhole: {mistake}\n"));
                return 2;
            }
        }
    }
    // Inside one of the app's tabs: say so. An empty value is the same as none.
    let from = super::tab_var("WORMHOLE_TAB_ID").filter(|tab| !tab.trim().is_empty());
    let request = Request::new(action, from);
    let socket = super::tab_var("WORMHOLE_SOCKET").map(PathBuf::from).unwrap_or_else(super::socket_path);
    if matches!(request.action, Action::Events(_)) {
        return watch(&socket, &request, json, home.as_deref());
    }
    let reply = match exchange(&socket, &request, answer_wait(&request.action)) {
        Ok(reply) => reply,
        Err(Exchange::NotRunning) => {
            // Terminals that outlive the app can still be listed from their hosts' files.
            if let Some(text) = sessions::offline(&request.action, json) {
                print(&text);
                return 3;
            }
            return explain(&Exchange::NotRunning);
        }
        Err(err) => return explain(&err),
    };
    if reply.version != PROTOCOL_VERSION && reply.error.is_none() {
        print_err(&format!(
            "Wormhole: the running Wormhole speaks protocol {} and this command line {PROTOCOL_VERSION}\n",
            reply.version
        ));
        return 2;
    }
    show(&request.action, &reply, json, home.as_deref(), numbers);
    exit_code_of(&request.action, &reply, cli.exit_code)
}

/// How long to wait for the app's answer: a moment, except for `wait` and `run`, which answer when they
/// are done: their own timeout, and a margin for the app to say it ran out.
fn answer_wait(action: &Action) -> Duration {
    let patient = match action {
        Action::Wait(spec) => spec.wait_for(),
        Action::Run(spec) => spec.wait_for(),
        _ => return REPLY_WAIT,
    };
    patient + Duration::from_secs(5)
}

/// The command line's exit code for `reply`: 0 when it worked, 124 when a `wait` or `run` ran out of
/// time (as `timeout(1)` does), the error's own otherwise; and for `run --exit-code` the command's.
fn exit_code_of(action: &Action, reply: &Reply, command_code: bool) -> i32 {
    if let Some(error) = &reply.error {
        let waits = matches!(action, Action::Wait(_) | Action::Run(_));
        return if waits && error.reason == Some(Reason::Timeout) { 124 } else { error.code.exit_code() };
    }
    if command_code && matches!(action, Action::Run(_)) {
        let ran = reply.result.clone().and_then(|result| serde_json::from_value::<Ran>(result).ok());
        return ran.and_then(|ran| ran.exit_code).unwrap_or(0);
    }
    0
}

/// Say why there was no reply, and the exit code that goes with it.
fn explain(err: &Exchange) -> i32 {
    match err {
        Exchange::NoAnswer => {
            print_err("Wormhole: the running Wormhole didn't answer (an older build, or still starting)\n");
        }
        Exchange::NotRunning | Exchange::Broken(_) => print_err(&format!("Wormhole: {}\n", err.words())),
    }
    3
}

/// Print a reply: the JSON object it is, or as text (an error to stderr).
fn show(action: &Action, reply: &Reply, json: bool, home: Option<&Path>, numbers: bool) {
    if json {
        print(&format!("{}\n", json_line(reply)));
    } else if let (Action::Run(_), None, Some(ran)) = (action, &reply.error, run_result(reply)) {
        // What the command printed is the output; how it went is not.
        let (output, note) = run_parts(&ran);
        print(&output);
        print_err(&note);
    } else {
        match render_with(action, reply, home, numbers) {
            Ok(text) => print(&text),
            Err(words) => print_err(&format!("Wormhole: {words}\n")),
        }
    }
}

fn run_result(reply: &Reply) -> Option<Ran> {
    serde_json::from_value(reply.result.clone()?).ok()
}

/// `events`: print the stream until it ends. Ctrl-c ends it with 0, as does Wormhole quitting; being shut
/// out or too slow is 1.
fn watch(socket: &Path, request: &Request, json: bool, home: Option<&Path>) -> i32 {
    if let Err(err) = crate::platform::quit_signal::exit_on_interrupt() {
        tracing::warn!("ctrl-c will end the stream with the signal's own code: {err}");
    }
    let style = events::Style::for_stdout();
    let on_reply = |text: &str, reply: Reply| {
        if reply.error.is_some() || json {
            // A refusal goes where errors go, whichever format.
            if reply.error.is_some() && !json {
                show(&request.action, &reply, false, home, false);
            } else {
                print(&format!("{text}\n"));
            }
            return match reply.error {
                Some(error) => ControlFlow::Break(error.code.exit_code()),
                None => ControlFlow::Continue(()),
            };
        }
        let subscribed = reply.result.and_then(|result| serde_json::from_value::<Subscribed>(result).ok());
        if let Some(subscribed) = subscribed {
            print(&events::header(&subscribed));
        }
        ControlFlow::Continue(())
    };
    let on_line = |text: &str| {
        let Some(parsed) = StreamLine::parse(text) else { return ControlFlow::Continue(()) };
        if json {
            print(&format!("{text}\n"));
        } else if let Some(shown) = events::line(&parsed, home, style) {
            print(&shown);
        }
        match parsed {
            StreamLine::Notice(crate::ipc::control::event::Notice::Closed { reason }) => {
                ControlFlow::Break(events::exit_code(reason))
            }
            _ => ControlFlow::Continue(()),
        }
    };
    match exchange_stream(socket, request, on_reply, on_line) {
        Ok(StreamEnd::Stopped(code)) => code,
        // Wormhole saying it is quitting is a line (0); a connection that just ends is not.
        Ok(StreamEnd::Hangup) => {
            print_err("Wormhole: the stream ended without a closing line (Wormhole quit, or stopped)\n");
            1
        }
        Err(err) => explain(&err),
    }
}

/// `send-text -`: the text piped in, up to the cap (and a byte, so the app can say it is over).
fn read_stdin_text() -> Result<String, String> {
    let mut text = String::new();
    io::stdin()
        .take(SEND_TEXT_MAX_BYTES as u64 + 1)
        .read_to_string(&mut text)
        .map_err(|e| format!("reading stdin: {e}"))?;
    if text.trim().is_empty() {
        return Err("nothing on stdin to send".into());
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{answer_wait, exit_code_of};
    use crate::ipc::control::{Action, Error, ErrorCode, Ran, Reason, Reply, RunSpec, Target, WaitSpec};

    fn wait(timeout_ms: Option<u64>) -> Action {
        Action::Wait(WaitSpec {
            target: Target::Terminal(3),
            status: vec![],
            text: None,
            gone: None,
            stable_ms: None,
            block_done: true,
            timeout_ms,
        })
    }

    fn run(timeout_ms: Option<u64>) -> Action {
        Action::Run(RunSpec { target: Target::Terminal(3), command: "make".into(), timeout_ms })
    }

    fn ran(exit_code: Option<i32>) -> Reply {
        Reply::ok(Ran {
            terminal: 3,
            block: 4,
            exit_code,
            duration_ms: 10,
            output: None,
            truncated: false,
            output_hidden: None,
        })
    }

    #[test]
    fn a_wait_or_run_is_waited_for_as_long_as_it_was_given_and_a_little_more() {
        assert_eq!(answer_wait(&Action::List), Duration::from_secs(10));
        assert_eq!(answer_wait(&wait(None)), Duration::from_secs(35));
        assert_eq!(answer_wait(&wait(Some(90_000))), Duration::from_secs(95));
        assert_eq!(answer_wait(&run(None)), Duration::from_secs(605));
        assert_eq!(answer_wait(&run(Some(3_600_000))), Duration::from_secs(3605));
    }

    #[test]
    fn a_timeout_is_124_and_the_commands_own_code_is_wanted_only_when_asked_for() {
        let timeout: Reply = Error::new(ErrorCode::Failed, "slow").because(Reason::Timeout).into();
        assert_eq!(exit_code_of(&wait(None), &timeout, false), 124);
        assert_eq!(exit_code_of(&run(None), &timeout, true), 124);
        // Another verb's failure is the usual 1; a refusal of a run is too.
        assert_eq!(exit_code_of(&Action::List, &timeout, false), 1);
        let refused: Reply = Error::new(ErrorCode::Refused, "no").into();
        assert_eq!(exit_code_of(&run(None), &refused, true), 1);
        let mistake: Reply = Error::new(ErrorCode::BadRequest, "bad").into();
        assert_eq!(exit_code_of(&wait(None), &mistake, false), 2);
        // A run that ran is 0 whatever the command's code, unless the command's code is what is asked.
        assert_eq!(exit_code_of(&run(None), &ran(Some(101)), false), 0);
        assert_eq!(exit_code_of(&run(None), &ran(Some(101)), true), 101);
        assert_eq!(exit_code_of(&run(None), &ran(Some(0)), true), 0);
        assert_eq!(exit_code_of(&run(None), &ran(None), true), 0);
        assert_eq!(exit_code_of(&wait(None), &Reply::ok(serde_json::json!({})), true), 0);
    }
}
