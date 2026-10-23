//! `Wormhole hook …`: the command agents' hooks run to report to the app; `Wormhole statusline`:
//! what Claude Code's status line runs to hand over its plan usage limits and the terminal's
//! model and context window.

use std::io::{IsTerminal as _, Read, Write};
use std::time::Duration;

use serde_json::Value;

use super::Message;
use crate::agents::{AgentId, integrations};
use crate::platform::socket::Stream;

/// What the binary was started to do, from its arguments.
#[derive(Debug, PartialEq, Eq)]
pub enum Command<'a> {
    /// `hook …`: report an agent's event (the arguments after `hook`).
    Hook(&'a [String]),
    /// `statusline`: hand over Claude Code's plan usage, model and context window.
    StatusLine,
    /// `list`, `status`, `notify`, `send-text`, `new-terminal` or `help`: a control request to
    /// the running app (the arguments from the verb on; `control::run`).
    Control(&'a [String]),
    /// `welcome [-- <shell…>]`: the first-run welcome, a program in the terminal (the arguments
    /// after `welcome`; `welcome::run`).
    Welcome(&'a [String]),
    /// No command: start the app, or bring the running one forward.
    App,
    /// A command this build doesn't know: most likely one a newer build installed into an agent's
    /// settings, run by this older one. It must not count as a launch, or every run of it brings
    /// the running app to the front (a status line runs many times a minute).
    Unknown(&'a str),
}

/// What `args` (as `std::env::args`, the program first) ask for. A `-psn_…` argument (the
/// process serial number older macOS versions pass when the Finder launches an app) is a launch.
pub fn command(args: &[String]) -> Command<'_> {
    match args.get(1).map(String::as_str) {
        Some("hook") => Command::Hook(args.get(2..).unwrap_or_default()),
        Some("statusline") => Command::StatusLine,
        Some("welcome") => Command::Welcome(args.get(2..).unwrap_or_default()),
        Some(verb) if super::control::is_verb(verb) => Command::Control(&args[1..]),
        None => Command::App,
        Some(arg) if arg.starts_with("-psn_") => Command::App,
        Some(arg) => Command::Unknown(arg),
    }
}

/// `wormhole hook [--agent <id>] <Event> [payload]`, called by an agent's
/// hook. The payload is JSON on stdin (most agents) or the last argument
/// (Codex `notify`). Never fails loudly; a hook must not break the agent.
pub fn run_cli(args: &[String]) {
    let (Some(socket), Some(tab)) = (super::tab_var("WORMHOLE_SOCKET"), super::tab_var("WORMHOLE_TAB_ID"))
    else {
        return;
    };
    let invocation = parse_cli(args);
    let payload = match invocation.payload {
        Some(arg) => serde_json::from_str(&arg).unwrap_or(Value::Null),
        None => read_stdin_payload(),
    };
    // Model-response telemetry does not change whether an agent is busy or waiting.
    if !(invocation.agent == AgentId::Gemini && invocation.event == "AfterModel") {
        let message = hook_message(tab, invocation.agent, &invocation.event, &payload);
        send(&socket, &message);
    }
    let codex_home = std::env::var_os("CODEX_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| integrations::home().join(".codex"));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64);
    if let Some(usage) =
        crate::agents::usage::from_hook(invocation.agent, &invocation.event, &payload, &codex_home, now)
    {
        send(&socket, &Message::Usage { usage });
    }
}

/// `Wormhole statusline`, which Claude Code's status line entry runs with its status JSON on
/// stdin: the plan usage limits in it, and the terminal's model and context window, go to the
/// app. Prints nothing (the user's own status line command, if any, prints the line) and never
/// fails loudly.
pub fn run_status_line_cli() {
    let Some(socket) = super::tab_var("WORMHOLE_SOCKET") else { return };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64);
    let tab = super::tab_var("WORMHOLE_TAB_ID");
    for message in status_line_messages(&read_stdin_payload(), tab, now) {
        send(&socket, &message);
    }
}

/// What a status line's JSON tells the app: the plan usage (the account's), then the gauge (this
/// terminal's, so only with the tab the status line runs in).
fn status_line_messages(input: &Value, tab: Option<String>, now: i64) -> Vec<Message> {
    let usage =
        crate::agents::usage::Usage::from_claude_status(input, now).map(|usage| Message::Usage { usage });
    let gauge = tab
        .zip(crate::agents::usage::Gauge::from_claude_status(input))
        .map(|(tab, gauge)| Message::Gauge { tab, gauge });
    usage.into_iter().chain(gauge).collect()
}

fn send(socket: &str, message: &Message) {
    if let Ok(mut stream) = Stream::connect(socket) {
        let _ = stream.set_write_timeout(Some(Duration::from_millis(500)));
        if let Ok(json) = serde_json::to_string(message) {
            let _ = writeln!(stream, "{json}");
        }
    }
}

/// The JSON payload on stdin, if the agent piped one. Never waits on a
/// terminal, and gives up quickly on a pipe that stays open.
fn read_stdin_payload() -> Value {
    if std::io::stdin().is_terminal() {
        return Value::Null;
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut input = String::new();
        let _ = std::io::stdin().take(1 << 20).read_to_string(&mut input);
        let _ = tx.send(input);
    });
    rx.recv_timeout(Duration::from_secs(1))
        .ok()
        .and_then(|input| serde_json::from_str(&input).ok())
        .unwrap_or(Value::Null)
}

#[derive(Debug, PartialEq)]
struct Invocation {
    agent: AgentId,
    event: String,
    payload: Option<String>,
}

fn parse_cli(args: &[String]) -> Invocation {
    let mut agent = AgentId::Claude;
    let mut rest = args;
    if let [flag, id, tail @ ..] = args
        && flag == "--agent"
    {
        agent = serde_json::from_value(Value::String(id.clone())).unwrap_or(AgentId::Claude);
        rest = tail;
    }
    Invocation { agent, event: rest.first().cloned().unwrap_or_default(), payload: rest.get(1).cloned() }
}

/// The event is the one on the command line (the payload's `hook_event_name` only for bare
/// invocations), with the payload's detail appended as `event:detail` so callers can tell a
/// question from a reminder (`integrations::describe`).
fn hook_message(tab: String, agent: AgentId, event: &str, payload: &Value) -> Message {
    let event = match event {
        "" => payload.get("hook_event_name").and_then(Value::as_str).unwrap_or_default(),
        event => event,
    };
    let (event, message) = integrations::describe(event, payload);
    Message::Hook { tab, agent, event, message }
}

#[cfg(test)]
mod tests {
    use super::{
        AgentId, Command, Invocation, Message, Value, command, hook_message, parse_cli, status_line_messages,
    };

    fn kinds(messages: &[Message]) -> Vec<&'static str> {
        messages
            .iter()
            .map(|message| match message {
                Message::Usage { .. } => "usage",
                Message::Gauge { .. } => "gauge",
                _ => "other",
            })
            .collect()
    }

    /// One status line run says what the account's limits are and what this terminal's agent is
    /// doing; the gauge needs the tab, which only a terminal of Wormhole's has.
    #[test]
    fn a_status_line_reports_usage_and_the_terminals_gauge() {
        let input = serde_json::json!({
            "model": { "display_name": "Opus" },
            "context_window": { "used_percentage": 41 },
            "rate_limits": { "five_hour": { "used_percentage": 25, "resets_at": 1_800_000_600 } }
        });
        let both = status_line_messages(&input, Some("7".into()), 1_800_000_000);
        assert_eq!(kinds(&both), ["usage", "gauge"]);
        assert!(matches!(&both[1], Message::Gauge { tab, .. } if tab == "7"));
        assert_eq!(kinds(&status_line_messages(&input, None, 1_800_000_000)), ["usage"]);
        let no_limits = serde_json::json!({ "model": { "display_name": "Opus" } });
        assert_eq!(kinds(&status_line_messages(&no_limits, Some("7".into()), 0)), ["gauge"]);
        assert!(status_line_messages(&Value::Null, Some("7".into()), 0).is_empty());
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// Claude's installed hooks run `hook <Event>` without `--agent`.
    #[test]
    fn a_bare_hook_call_is_claudes() {
        let parsed = parse_cli(&args(&["Stop"]));
        assert_eq!(parsed, Invocation { agent: AgentId::Claude, event: "Stop".into(), payload: None });
    }

    #[test]
    fn agent_and_argument_payload_parse() {
        let parsed = parse_cli(&args(&["--agent", "copilot", "notification", r#"{"a":1}"#]));
        assert_eq!(parsed.agent, AgentId::Copilot);
        assert_eq!(parsed.event, "notification");
        assert_eq!(parsed.payload.as_deref(), Some(r#"{"a":1}"#));
    }

    #[test]
    fn notification_types_and_messages_are_kept() {
        let payload = serde_json::json!({
            "hook_event_name": "Notification",
            "notification_type": "permission_prompt",
            "message": "Claude needs your permission to use Bash"
        });
        let Message::Hook { event, message, .. } = hook_message("1".into(), AgentId::Claude, "", &payload)
        else {
            panic!("not a hook message");
        };
        assert_eq!(event, "Notification:permission_prompt");
        assert_eq!(message.as_deref(), Some("Claude needs your permission to use Bash"));
    }

    #[test]
    fn opencode_hooks_are_attributed_to_opencode() {
        // The installed OpenCode plugin runs `hook --agent opencode <event>`.
        let parsed = parse_cli(&args(&["--agent", "opencode", "session.idle"]));
        assert_eq!(parsed.agent, AgentId::OpenCode);
        let wire = serde_json::to_string(&hook_message("1".into(), AgentId::OpenCode, "x", &Value::Null))
            .expect("serializes");
        assert!(wire.contains(r#""agent":"opencode""#), "{wire}");
    }

    #[test]
    fn unknown_agents_fall_back_to_claude_and_messages_default_it() {
        assert_eq!(parse_cli(&args(&["--agent", "nope", "Stop"])).agent, AgentId::Claude);
        let old: Message =
            serde_json::from_str(r#"{"kind":"hook","tab":"3","event":"Stop"}"#).expect("parses");
        assert!(matches!(old, Message::Hook { agent: AgentId::Claude, .. }));
    }

    #[test]
    fn only_a_bare_launch_starts_the_app() {
        assert_eq!(command(&args(&["Wormhole"])), Command::App);
        assert_eq!(command(&args(&["Wormhole", "-psn_0_12345"])), Command::App);
        assert_eq!(command(&args(&["Wormhole", "statusline"])), Command::StatusLine);
        let welcome = args(&["Wormhole", "welcome", "--", "/bin/zsh", "-l"]);
        assert_eq!(command(&welcome), Command::Welcome(&welcome[2..]));
        assert_eq!(command(&args(&["Wormhole", "welcome"])), Command::Welcome(&[]));
        let hook = args(&["Wormhole", "hook", "--agent", "codex", "Stop"]);
        assert_eq!(command(&hook), Command::Hook(&hook[2..]));
        assert_eq!(command(&args(&["Wormhole", "hook"])), Command::Hook(&[]));
        // The control command line, from the verb on.
        let list = args(&["Wormhole", "list", "--json"]);
        assert_eq!(command(&list), Command::Control(&list[1..]));
        for verb in ["status", "notify", "send-text", "new-terminal", "help", "--help", "-h"] {
            let words = args(&["Wormhole", verb]);
            assert_eq!(command(&words), Command::Control(&words[1..]), "{verb}");
        }
        // A command from a newer build, run by an older one: refused, never a second launch that
        // brings the running app forward.
        assert_eq!(command(&args(&["Wormhole", "usage"])), Command::Unknown("usage"));
        assert_eq!(command(&args(&["Wormhole", "--version"])), Command::Unknown("--version"));
    }
}
