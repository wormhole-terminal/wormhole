//! `Wormhole list | status | notify | send-text | new-terminal`: the command line's side of the
//! control protocol. The arguments are read into a [`Request`] (`parse`), the app's [`Reply`] is
//! printed as text or, with `--json`, as the one JSON object it is (`render`), and `run` does the
//! round trip over the socket. Folders are resolved here, against the caller's working directory
//! and home: the app cannot know where a script ran from.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{
    Action, Asked, ErrorCode, Listing, Opened, PROTOCOL_VERSION, Reached, Reply, Request,
    SEND_TEXT_MAX_BYTES, StatusInfo, Summary, TabInfo, Target, TerminalInfo,
};
use crate::ipc::Message;

/// What the arguments asked for.
#[derive(Debug, Clone, PartialEq)]
pub struct Cli {
    pub action: Action,
    /// Print the reply as JSON.
    pub json: bool,
    /// `send-text -`: the text is on stdin.
    pub stdin: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    Run(Cli),
    Help,
}

/// What the command is called when someone types it: the name of the script that `cli_tool` puts in
/// `PATH` for Wormhole's terminals and, on request, for any terminal. One constant, so the script's
/// file name, the usage text, Settings and the docs' examples can't disagree.
pub const COMMAND_NAME: &str = "wh";

/// The usage text, with the command by the name it is called by (`COMMAND_NAME`), not the binary's.
pub fn help() -> String {
    let wh = COMMAND_NAME;
    format!(
        "\
Wormhole: control the running app from the command line

  {wh} list
      Every window, terminal, tab and pane, with their ids.
  {wh} status [<target>]
      One terminal's status: the target's, or this tab's inside a terminal; a summary otherwise.
  {wh} notify [<target>] [--title <text>] <body>...
      A notification about a terminal (this tab without a target), in your words.
  {wh} send-text <target> [--submit] <text>... | -
      Type into a terminal, as a paste (`-` reads the text from stdin); --submit presses Return.
  {wh} new-terminal [--folder <path>] [--command <command>] [--window <id>] [--worktree <branch>]
      A new terminal in the front window (or --window), in a folder, running a command in its shell.

  {wh} welcome [-- <program> [<args>...]]
      The first-run welcome, in this terminal; then it starts <program> (your shell) here.

  <target>: --terminal <id> | --tab <id> | --pane <id>, with the ids `list` prints.
  --json prints the reply as one JSON object; the exit code is 0 when it worked, 1 when the app
  would not or could not, 2 for a mistake in the arguments, 3 when Wormhole isn't running.

Typing into terminals and opening them are off until Settings → Coding agents → \"Let the command
line type and open terminals\" is on; listing, status and notify always work. Inside a terminal,
WORMHOLE_TAB_ID says which tab asked.

{wh} runs in Wormhole's own terminals; for any other terminal, Settings → Coding agents → \"Command
line tool\" installs it.
"
    )
}

/// Whether `word` as the first argument is this command line's (`Wormhole list …`).
pub fn is_verb(word: &str) -> bool {
    matches!(word, "list" | "status" | "notify" | "send-text" | "new-terminal" | "help" | "--help" | "-h")
}

/// Read the words after the program name (`list --json`, `send-text --tab 3 hello`) into what to
/// do. `cwd` and `home` resolve a relative folder and a leading `~`.
pub fn parse(args: &[String], cwd: &Path, home: Option<&Path>) -> Result<Parsed, String> {
    let (verb, rest) = match args.split_first() {
        Some((verb, rest)) => (verb.as_str(), rest),
        None => return Ok(Parsed::Help),
    };
    if matches!(verb, "help" | "--help" | "-h") {
        return Ok(Parsed::Help);
    }
    let mut json = false;
    let mut target = None;
    let mut title = None;
    let mut submit = false;
    let mut folder = None;
    let mut command = None;
    let mut worktree = None;
    let mut window = None;
    let mut words: Vec<&str> = Vec::new();
    let mut args = rest.iter().map(String::as_str);
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| args.next().ok_or_else(|| format!("{flag} needs a value"));
        let id = |flag: &str, value: &str| {
            value.parse::<i64>().map_err(|_| format!("{flag} takes an id (a number), not {value:?}"))
        };
        match arg {
            "--json" => json = true,
            "--help" | "-h" => return Ok(Parsed::Help),
            "--terminal" | "--tab" | "--pane" => {
                let value = value(arg)?;
                let id = id(arg, value)?;
                let one = match arg {
                    "--terminal" => Target::Terminal(id),
                    "--tab" => Target::Tab(id),
                    _ => Target::Pane(id),
                };
                if target.replace(one).is_some() {
                    return Err("name one target: --terminal, --tab or --pane".into());
                }
            }
            "--title" if verb == "notify" => title = Some(value(arg)?.to_string()),
            "--submit" if verb == "send-text" => submit = true,
            "--folder" if verb == "new-terminal" => folder = Some(value(arg)?.to_string()),
            "--command" if verb == "new-terminal" => command = Some(value(arg)?.to_string()),
            "--worktree" if verb == "new-terminal" => worktree = Some(value(arg)?.to_string()),
            "--window" if verb == "new-terminal" => {
                let value = value(arg)?;
                window = Some(id(arg, value)?);
            }
            flag if flag.starts_with('-') && flag != "-" => {
                return Err(format!("unknown option {flag} for {verb}"));
            }
            word => words.push(word),
        }
    }
    let text = words.join(" ");
    let action = match verb {
        "list" => Action::List,
        "status" => Action::Status { target },
        "notify" => {
            if text.trim().is_empty() && title.as_deref().is_none_or(|t| t.trim().is_empty()) {
                return Err("notify needs something to say: a body, or --title".into());
            }
            Action::Notify { target, title: title.unwrap_or_default(), body: text }
        }
        "send-text" => {
            let Some(target) = target else {
                return Err("send-text needs a target: --terminal, --tab or --pane".into());
            };
            if text.is_empty() {
                return Err("send-text needs the text to type (or - for stdin)".into());
            }
            Action::SendText { target, text, submit }
        }
        "new-terminal" => {
            if !text.is_empty() {
                return Err(format!(
                    "new-terminal takes options only, not {text:?} (did you mean --folder?)"
                ));
            }
            let folder = folder.map(|folder| resolve_folder(&folder, cwd, home));
            Action::NewTerminal { folder, command, worktree, window }
        }
        other => return Err(format!("unknown command {other:?}")),
    };
    if target.is_some() && matches!(action, Action::List | Action::NewTerminal { .. }) {
        return Err(format!("{verb} takes no target"));
    }
    let stdin = matches!(&action, Action::SendText { text, .. } if text == "-");
    Ok(Parsed::Run(Cli { action, json, stdin }))
}

/// `folder` as an absolute path: `~` is `home`, a relative path is under `cwd`. Nothing is
/// checked on disk here: the app says whether the folder exists.
fn resolve_folder(folder: &str, cwd: &Path, home: Option<&Path>) -> String {
    let path = match (folder, home) {
        ("~", Some(home)) => home.to_path_buf(),
        (folder, Some(home)) if folder.starts_with("~/") => home.join(&folder[2..]),
        (folder, _) if Path::new(folder).is_absolute() => PathBuf::from(folder),
        (folder, _) => cwd.join(folder),
    };
    path.to_string_lossy().into_owned()
}

/// The reply as a person reads it (`--json` prints the object instead). `home` shortens folders
/// under it to `~`. An error comes back as `Err` with its words, for stderr.
pub fn render(reply: &Reply, home: Option<&Path>) -> Result<String, String> {
    if let Some(error) = &reply.error {
        return Err(format!("{} ({})", error.message, code_name(error.code)));
    }
    let result = reply.result.clone().unwrap_or(serde_json::Value::Null);
    let tilde = |folder: &Option<String>| folder.as_deref().map(|f| shorten(f, home)).unwrap_or_default();
    if let Ok(listing) = serde_json::from_value::<Listing>(result.clone()) {
        let mut out = String::new();
        for window in &listing.windows {
            out.push_str(&format!("window {}{}\n", window.id, if window.front { "  front" } else { "" }));
            for terminal in &window.terminals {
                out.push_str(&terminal_line(terminal, home));
                for tab in &terminal.tabs {
                    out.push_str(&tab_line(tab));
                }
            }
        }
        if listing.windows.is_empty() {
            out.push_str("no windows\n");
        }
        return Ok(out);
    }
    if let Ok(status) = serde_json::from_value::<StatusInfo>(result.clone()) {
        let mut out = format!("window {}\n", status.window);
        out.push_str(&terminal_line(&status.terminal, home));
        for tab in status.terminal.tabs.iter().filter(|tab| tab.id == status.tab) {
            out.push_str(&tab_line(tab));
        }
        return Ok(out);
    }
    if let Ok(opened) = serde_json::from_value::<Opened>(result.clone()) {
        let worktree = opened.worktree.as_deref().map(|w| format!(" on worktree {w}")).unwrap_or_default();
        return Ok(format!(
            "opened terminal {} (tab {}, pane {}) in window {} at {}{worktree}\n",
            opened.terminal,
            opened.tab,
            opened.pane,
            opened.window,
            tilde(&opened.folder)
        ));
    }
    if let Ok(summary) = serde_json::from_value::<Summary>(result.clone()) {
        return Ok(format!(
            "{} {}, {} {}: {} needs you, {} working, {} finished, {} failed, {} idle, {} exited\n",
            summary.windows,
            plural(summary.windows, "window"),
            summary.terminals,
            plural(summary.terminals, "terminal"),
            summary.needs_you,
            summary.working,
            summary.finished,
            summary.failed,
            summary.idle,
            summary.exited
        ));
    }
    if let Ok(asked) = serde_json::from_value::<Asked>(result.clone()) {
        return Ok(format!(
            "asked in window {}: confirm the worktree {} at {} there; the terminal opens once you do\n",
            asked.window,
            asked.branch,
            shorten(&asked.folder, home)
        ));
    }
    if let Ok(reached) = serde_json::from_value::<Reached>(result) {
        return Ok(format!("tab {}\n", reached.tab));
    }
    Ok("ok\n".into())
}

fn terminal_line(terminal: &TerminalInfo, home: Option<&Path>) -> String {
    let mut parts =
        vec![format!("  terminal {}  {:?}", terminal.id, terminal.name), terminal.status.word().to_string()];
    if let Some(agent) = &terminal.agent {
        parts.push(agent.clone());
    }
    if let Some(folder) = &terminal.folder {
        parts.push(shorten(folder, home));
    }
    if let Some(branch) = &terminal.branch {
        match terminal.changed {
            Some(changed) if changed > 0 => parts.push(format!("{branch} ±{changed}")),
            _ => parts.push(branch.clone()),
        }
    }
    if terminal.pinned {
        parts.push("pinned".into());
    }
    if terminal.selected {
        parts.push("selected".into());
    }
    format!("{}\n", parts.join("  "))
}

fn tab_line(tab: &TabInfo) -> String {
    let mut parts =
        vec![format!("    tab {}  {:?}", tab.id, tab.name), tab.status.word().to_string(), tab.shell.clone()];
    if let Some(command) = &tab.command {
        parts.push(format!("running {command}"));
    }
    if let Some(message) = &tab.message {
        parts.push(format!("{message:?}"));
    }
    if tab.selected {
        parts.push("selected".into());
    }
    let panes: Vec<String> = tab.panes.iter().map(|pane| pane.id.to_string()).collect();
    parts.push(format!("{} {}", plural(panes.len(), "pane"), panes.join(" ")));
    format!("{}\n", parts.join("  "))
}

fn plural(n: usize, word: &str) -> String {
    if n == 1 { word.to_string() } else { format!("{word}s") }
}

/// `folder` with `home` as `~`.
fn shorten(folder: &str, home: Option<&Path>) -> String {
    match home.map(|home| home.to_string_lossy().into_owned()) {
        Some(home) if folder == home => "~".into(),
        Some(home) if folder.starts_with(&format!("{home}/")) => format!("~{}", &folder[home.len()..]),
        _ => folder.to_string(),
    }
}

/// An error code as the text output names it (`not_allowed`).
fn code_name(code: ErrorCode) -> String {
    serde_json::to_value(code).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

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
    let mut stream = UnixStream::connect(socket).map_err(|_| Exchange::NotRunning)?;
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
    let cwd = std::env::current_dir().unwrap_or_default();
    let home = std::env::var_os("HOME").map(PathBuf::from);
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
    let from = crate::ipc::tab_var("WORMHOLE_TAB_ID").filter(|tab| !tab.trim().is_empty());
    let request = Request::new(action, from);
    let socket =
        crate::ipc::tab_var("WORMHOLE_SOCKET").map(PathBuf::from).unwrap_or_else(super::super::socket_path);
    let reply = match exchange(&socket, &request, REPLY_WAIT) {
        Ok(reply) => reply,
        Err(Exchange::NotRunning) => {
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

/// Write to stdout as a command line does; a closed pipe (`| head`) is not an error worth saying.
fn print(text: &str) {
    let _ = io::stdout().lock().write_all(text.as_bytes());
}

fn print_err(text: &str) {
    let _ = io::stderr().lock().write_all(text.as_bytes());
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Cli, Parsed, parse, render, shorten};
    use crate::ipc::control::{
        Action, ErrorCode, Listing, Opened, PaneInfo, Reached, Reply, StatusInfo, Summary, TabInfo, Target,
        TerminalInfo, TerminalState, WindowInfo,
    };

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn parsed(list: &[&str]) -> Cli {
        match parse(&words(list), Path::new("/work/here"), Some(Path::new("/Users/me"))) {
            Ok(Parsed::Run(cli)) => cli,
            other => panic!("{list:?}: {other:?}"),
        }
    }

    fn mistake(list: &[&str]) -> String {
        match parse(&words(list), Path::new("/work/here"), Some(Path::new("/Users/me"))) {
            Err(mistake) => mistake,
            other => panic!("{list:?} should be refused: {other:?}"),
        }
    }

    #[test]
    fn every_verb_parses_with_its_options() {
        assert_eq!(parsed(&["list"]), Cli { action: Action::List, json: false, stdin: false });
        assert!(parsed(&["list", "--json"]).json);
        assert_eq!(parsed(&["status"]).action, Action::Status { target: None });
        assert_eq!(
            parsed(&["status", "--terminal", "3"]).action,
            Action::Status { target: Some(Target::Terminal(3)) }
        );
        assert_eq!(
            parsed(&["notify", "--tab", "9", "--title", "Deploy", "done", "in", "3s"]).action,
            Action::Notify {
                target: Some(Target::Tab(9)),
                title: "Deploy".into(),
                body: "done in 3s".into()
            }
        );
        assert_eq!(
            parsed(&["notify", "--title", "Ready"]).action,
            Action::Notify { target: None, title: "Ready".into(), body: String::new() },
            "a title alone will do"
        );
        assert_eq!(
            parsed(&["send-text", "--pane", "4", "--submit", "run", "the", "tests"]).action,
            Action::SendText { target: Target::Pane(4), text: "run the tests".into(), submit: true }
        );
        let piped = parsed(&["send-text", "--tab", "2", "-"]);
        assert!(piped.stdin, "- reads stdin");
        assert_eq!(
            parsed(&[
                "new-terminal",
                "--folder",
                "~/src/app",
                "--command",
                "claude",
                "--window",
                "2",
                "--worktree",
                "fix"
            ])
            .action,
            Action::NewTerminal {
                folder: Some("/Users/me/src/app".into()),
                command: Some("claude".into()),
                worktree: Some("fix".into()),
                window: Some(2),
            }
        );
        assert_eq!(
            parsed(&["new-terminal", "--folder", "api"]).action,
            Action::NewTerminal {
                folder: Some("/work/here/api".into()),
                command: None,
                worktree: None,
                window: None
            },
            "relative to where the command ran"
        );
        assert_eq!(
            parsed(&["new-terminal", "--folder", "~"]).action,
            Action::NewTerminal {
                folder: Some("/Users/me".into()),
                command: None,
                worktree: None,
                window: None
            }
        );
    }

    #[test]
    fn help_is_asked_for_in_every_usual_way() {
        for list in [&[][..], &["help"], &["--help"], &["-h"], &["list", "--help"], &["send-text", "-h"]] {
            assert_eq!(parse(&words(list), Path::new("/"), None).expect("parses"), Parsed::Help, "{list:?}");
        }
        let help = super::help();
        assert!(help.contains(&format!("{} list", super::COMMAND_NAME)) && help.contains("--json"));
        // The usage names the command people type, never the binary's own name for it.
        assert!(!help.contains("Wormhole list") && !help.contains("Wormhole send-text"));
    }

    #[test]
    fn mistakes_are_named() {
        assert!(mistake(&["frobnicate"]).contains("unknown command"));
        assert!(mistake(&["send-text", "hello"]).contains("needs a target"));
        assert!(mistake(&["send-text", "--tab", "3"]).contains("needs the text"));
        assert!(mistake(&["send-text", "--tab", "x", "hi"]).contains("takes an id"));
        assert!(mistake(&["send-text", "--tab", "3", "--pane", "4", "hi"]).contains("one target"));
        assert!(mistake(&["notify"]).contains("something to say"));
        assert!(mistake(&["notify", "--title"]).contains("needs a value"));
        assert!(mistake(&["list", "--tab", "3"]).contains("takes no target"));
        assert!(mistake(&["new-terminal", "/work/app"]).contains("--folder"));
        assert!(mistake(&["list", "--submit"]).contains("unknown option"), "an option of another verb");
        assert!(mistake(&["status", "--bogus"]).contains("unknown option"));
    }

    fn tab(id: i64, name: &str, status: TerminalState, selected: bool) -> TabInfo {
        TabInfo {
            id,
            name: name.into(),
            status,
            agent: None,
            message: None,
            folder: Some("/Users/me/src/api".into()),
            shell: "zsh".into(),
            command: None,
            since_seconds: 12,
            selected,
            panes: vec![PaneInfo { id }],
        }
    }

    fn terminal() -> TerminalInfo {
        TerminalInfo {
            id: 3,
            name: "api".into(),
            status: TerminalState::NeedsYou,
            agent: Some("claude".into()),
            folder: Some("/Users/me/src/api".into()),
            branch: Some("main".into()),
            changed: Some(2),
            pinned: true,
            selected: true,
            tabs: vec![
                TabInfo {
                    message: Some("Allow Bash?".into()),
                    agent: Some("claude".into()),
                    ..tab(3, "claude", TerminalState::NeedsYou, true)
                },
                TabInfo {
                    command: Some("npm run dev".into()),
                    ..tab(5, "npm run dev", TerminalState::Working, false)
                },
            ],
        }
    }

    #[test]
    fn a_listing_reads_as_an_indented_tree() {
        let listing =
            Listing { windows: vec![WindowInfo { id: 1, front: true, terminals: vec![terminal()] }] };
        let text = render(&Reply::ok(listing), Some(Path::new("/Users/me"))).expect("renders");
        assert_eq!(
            text,
            "window 1  front\n\
             \x20 terminal 3  \"api\"  needs you  claude  ~/src/api  main ±2  pinned  selected\n\
             \x20   tab 3  \"claude\"  needs you  zsh  \"Allow Bash?\"  selected  pane 3\n\
             \x20   tab 5  \"npm run dev\"  working  zsh  running npm run dev  pane 5\n"
        );
        let empty = render(&Reply::ok(Listing::default()), None).expect("renders");
        assert_eq!(empty, "no windows\n");
    }

    #[test]
    fn the_other_results_read_as_a_line_each() {
        let home = Some(Path::new("/Users/me"));
        let status = StatusInfo { window: 1, terminal: terminal(), tab: 5 };
        let text = render(&Reply::ok(status), home).expect("renders");
        assert!(text.starts_with("window 1\n  terminal 3  \"api\""), "{text}");
        assert!(text.contains("tab 5") && !text.contains("tab 3  "), "only the tab asked about: {text}");
        let summary = Summary::of(2, [TerminalState::Working, TerminalState::Idle, TerminalState::NeedsYou]);
        assert_eq!(
            render(&Reply::ok(summary), home).expect("renders"),
            "2 windows, 3 terminals: 1 needs you, 1 working, 0 finished, 0 failed, 1 idle, 0 exited\n"
        );

        let one = Summary::of(1, [TerminalState::Idle]);
        assert!(render(&Reply::ok(one), home).expect("renders").starts_with("1 window, 1 terminal:"));
        let opened = Opened {
            window: 1,
            terminal: 8,
            tab: 8,
            pane: 8,
            folder: Some("/Users/me/src/api".into()),
            worktree: Some("fix/login".into()),
        };
        assert_eq!(
            render(&Reply::ok(opened), home).expect("renders"),
            "opened terminal 8 (tab 8, pane 8) in window 1 at ~/src/api on worktree fix/login\n"
        );
        assert_eq!(render(&Reply::ok(Reached { tab: 4 }), home).expect("renders"), "tab 4\n");
        let error = render(&Reply::error(ErrorCode::NotAllowed, "typing is off"), home);
        assert_eq!(error, Err("typing is off (not_allowed)".into()));
    }

    #[test]
    fn home_is_shortened_to_a_tilde() {
        let home = Some(Path::new("/Users/me"));
        assert_eq!(shorten("/Users/me", home), "~");
        assert_eq!(shorten("/Users/me/src", home), "~/src");
        assert_eq!(shorten("/Users/meh/src", home), "/Users/meh/src", "not a prefix of a longer name");
        assert_eq!(shorten("/tmp", home), "/tmp");
        assert_eq!(shorten("/Users/me/src", None), "/Users/me/src");
    }
}
