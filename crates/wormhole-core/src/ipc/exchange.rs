//! The command line's round trip: `Wormhole <verb> …` sends one request over the control socket and
//! prints the app's reply (`run`), and the same exchange serves the welcome's terminal (`exchange`). The
//! text of the verbs, their parsing and the printing of a reply are `control`'s; this is the half that
//! talks, and the only one that names [`Message`], so `control` (the protocol's types, which `Message`
//! carries) stays below it.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::Shutdown;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::Message;
use super::control::Parsed;
use super::control::sessions;
use super::control::{
    Action, PROTOCOL_VERSION, Reply, Request, SEND_TEXT_MAX_BYTES, help, parse, print, print_err, render,
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
    match BufReader::new(stream).take(1 << 20).read_line(&mut answer) {
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

/// `Wormhole <verb> …`: do the round trip and print the answer. Returns the exit code.
pub fn run(args: &[String]) -> i32 {
    // Session hosts are not the app's: this works from their files, with the app closed too.
    if args.first().map(String::as_str) == Some("sessions") {
        return sessions::run(&args[1..]);
    }
    let cwd = std::env::current_dir().unwrap_or_default();
    let home = paths::home();
    let cli = match parse(args, &cwd, home.as_deref()) {
        Ok(Parsed::Help) => {
            print(&help());
            return 0;
        }
        Ok(Parsed::Run(cli)) => cli,
        Err(mistake) => {
            print_err(&format!("Wormhole: {mistake}\n\n{}", help()));
            return 2;
        }
    };
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
    let reply = match exchange(&socket, &request, REPLY_WAIT) {
        Ok(reply) => reply,
        Err(Exchange::NotRunning) => {
            // Terminals that outlive the app can still be listed from their hosts' files.
            if let Some(text) = sessions::offline(&request.action, cli.json) {
                print(&text);
                return 3;
            }
            print_err("Wormhole: Wormhole isn't running\n");
            return 3;
        }
        Err(Exchange::NoAnswer) => {
            print_err("Wormhole: the running Wormhole didn't answer (an older build, or still starting)\n");
            return 3;
        }
        Err(Exchange::Broken(why)) => {
            print_err(&format!("Wormhole: {why}\n"));
            return 3;
        }
    };
    if reply.version != PROTOCOL_VERSION && reply.error.is_none() {
        print_err(&format!(
            "Wormhole: the running Wormhole speaks protocol {} and this command line {PROTOCOL_VERSION}\n",
            reply.version
        ));
        return 2;
    }
    if cli.json {
        print(&format!("{}\n", serde_json::to_string(&reply).unwrap_or_default()));
    } else {
        match render(&reply, home.as_deref()) {
            Ok(text) => print(&text),
            Err(words) => print_err(&format!("Wormhole: {words}\n")),
        }
    }
    reply.error.as_ref().map_or(0, |error| error.code.exit_code())
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
