//! `wh events` as text: one line for each event, aligned and, on a terminal, coloured. Everything a
//! terminal or a program said (titles, folders, commands, names) goes through [`for_terminal`]
//! first: text read from a terminal must not be able to drive the one it is printed in.

use std::fmt::Write as _;
use std::path::Path;

use super::render::shorten;
use crate::ipc::control::TerminalState;
use crate::ipc::control::event::{
    BlockPhase, Body, CloseReason, Event, EventLine, Notice, StreamLine, Subscribed,
};
use crate::ipc::control::printable::for_terminal;
use crate::platform::clock::local_time_of_day;

/// Whether to colour, and the colours: a small fixed set of SGR codes for states, which is the
/// command line's own output and not the app's interface, so no theme is involved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Style {
    pub(crate) color: bool,
}

impl Style {
    /// Colour only when stdout is a terminal and `NO_COLOR` is not set (https://no-color.org).
    pub(crate) fn for_stdout() -> Self {
        use std::io::IsTerminal as _;
        Self {
            color: std::io::stdout().is_terminal()
                && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty()),
        }
    }

    fn paint(self, code: &str, text: &str) -> String {
        if self.color { format!("\x1b[{code}m{text}\x1b[0m") } else { text.to_string() }
    }

    fn state(self, state: TerminalState) -> String {
        let code = match state {
            TerminalState::NeedsYou => "33",
            TerminalState::Failed => "31",
            TerminalState::Working => "34",
            TerminalState::Finished => "32",
            TerminalState::Idle | TerminalState::Exited | TerminalState::Other => "2",
        };
        self.paint(code, state.word())
    }
}

/// What a stream says when it starts: how much there is to watch, and from where.
pub(crate) fn header(subscribed: &Subscribed) -> String {
    let windows = subscribed.listing.windows.len();
    let terminals: usize = subscribed.listing.windows.iter().map(|w| w.terminals.len()).sum();
    format!(
        "watching {windows} {}, {terminals} {} from #{} (ctrl-c to stop)\n",
        if windows == 1 { "window" } else { "windows" },
        if terminals == 1 { "terminal" } else { "terminals" },
        subscribed.seq
    )
}

/// One line of the stream as text (with its newline), or nothing for a heartbeat. `home` shortens
/// folders to `~`.
pub(crate) fn line(line: &StreamLine, home: Option<&Path>, style: Style) -> Option<String> {
    match line {
        StreamLine::Event(event) => Some(event_text(event, home, style)),
        StreamLine::Notice(Notice::Heartbeat { .. }) => None,
        StreamLine::Notice(Notice::Lagged { dropped }) => {
            Some(style.paint("33", &format!("-- fell behind: {dropped} events were dropped\n")))
        }
        StreamLine::Notice(Notice::Closed { reason }) => {
            Some(format!("-- closed: {}\n", close_words(*reason)))
        }
    }
}

/// Why a stream closed, in words.
pub(crate) fn close_words(reason: CloseReason) -> &'static str {
    match reason {
        CloseReason::NotAllowed => "what it reads is no longer allowed",
        CloseReason::TooSlow => "it was not read fast enough",
        CloseReason::Quitting => "Wormhole is quitting",
        CloseReason::Limit => "a limit was reached",
        CloseReason::Other => "the app ended it",
    }
}

/// The command line's exit code once a stream is over for `reason`: 0 when Wormhole quit, 1 when it
/// was shut out or fell behind.
pub(crate) fn exit_code(reason: CloseReason) -> i32 {
    match reason {
        CloseReason::Quitting => 0,
        CloseReason::NotAllowed | CloseReason::TooSlow | CloseReason::Limit | CloseReason::Other => 1,
    }
}

fn event_text(line: &EventLine, home: Option<&Path>, style: Style) -> String {
    let event = &line.event;
    let time = local_time_of_day(line.at_ms).map_or_else(|| "--:--:--.---".to_string(), |t| t.to_string());
    let (kind, detail) = describe(event, home, style);
    let ids = match (event.window, event.terminal) {
        (Some(w), Some(t)) => format!("w{w} t{t}"),
        (Some(w), None) => format!("w{w}"),
        (None, Some(t)) => format!("t{t}"),
        (None, None) => String::new(),
    };
    let mut out = format!("{time}  {:<6}  {ids:<8}  {kind:<8}", format!("#{}", line.seq));
    if detail.is_empty() {
        out.truncate(out.trim_end().len());
    } else {
        let _ = write!(out, "  {detail}");
    }
    out.push('\n');
    out
}

/// The kind column and the rest of the line.
fn describe(event: &Event, home: Option<&Path>, style: Style) -> (&'static str, String) {
    let quoted = |text: &str| format!("\"{}\"", for_terminal(text));
    match &event.body {
        Body::Status { status, was, agent, message } => {
            let mut out = format!("{} → {}", style.state(*was), style.state(*status));
            if let Some(agent) = agent {
                let _ = write!(out, "  {}", for_terminal(agent));
            }
            if let Some(message) = message {
                let _ = write!(out, "  {}", quoted(message));
            }
            ("status", out)
        }
        Body::Title { title, name } => {
            let title = title.as_deref().map_or_else(|| "(no title)".to_string(), quoted);
            ("title", format!("{title}  named {}", quoted(name)))
        }
        Body::Folder { folder } => (
            "folder",
            folder
                .as_deref()
                .map_or_else(|| "(none)".to_string(), |f| for_terminal(&shorten(f, home)).into_owned()),
        ),
        Body::Bell => ("bell", String::new()),
        Body::Block { block, phase, exit_code, took_ms, command } => {
            let mut out = match phase {
                BlockPhase::Started => "started".to_string(),
                BlockPhase::Finished | BlockPhase::Other => {
                    let mut done = "finished".to_string();
                    if let Some(code) = exit_code {
                        let words = code.to_string();
                        done.push(' ');
                        done.push_str(&style.paint(if *code == 0 { "32" } else { "31" }, &words));
                    }
                    if let Some(took) = took_ms {
                        let _ = write!(
                            done,
                            "  {}",
                            wormhole_term::took_text(std::time::Duration::from_millis(*took))
                        );
                    }
                    done
                }
            };
            match command {
                Some(command) => {
                    let _ = write!(out, "  {}", for_terminal(command));
                }
                None => {
                    let _ = write!(out, "  #{block}");
                }
            }
            ("block", out)
        }
        Body::Split { split, terminals, focused } => {
            if terminals.is_empty() {
                ("split", format!("{split} dissolved"))
            } else {
                let members: Vec<String> = terminals.iter().map(|t| format!("t{t}")).collect();
                let focus = focused.map(|f| format!("  focus t{f}")).unwrap_or_default();
                ("split", format!("{split}: {}{focus}", members.join(" ")))
            }
        }
        Body::TerminalOpened { folder } => (
            "terminal",
            match folder {
                Some(folder) => format!("opened  {}", for_terminal(&shorten(folder, home))),
                None => "opened".to_string(),
            },
        ),
        Body::TerminalClosed => ("terminal", "closed".to_string()),
        Body::TerminalMoved { from } => ("terminal", format!("moved from window {from}")),
        Body::Renamed { name } => (
            "terminal",
            match name {
                Some(name) => format!("renamed {}", quoted(name)),
                None => "renamed (named after what runs in it again)".to_string(),
            },
        ),
        Body::WindowOpened => ("window", "opened".to_string()),
        Body::WindowClosed => ("window", "closed".to_string()),
        Body::Other => ("?", "an event this Wormhole doesn't know".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Style, exit_code, header, line};
    use crate::ipc::control::event::{
        BlockPhase, Body, CloseReason, Event, EventLine, Notice, StreamLine, Subscribed,
    };
    use crate::ipc::control::{Listing, TerminalState, WindowInfo};

    const PLAIN: Style = Style { color: false };

    fn event(seq: u64, window: Option<i64>, terminal: Option<i64>, body: Body) -> StreamLine {
        StreamLine::Event(EventLine {
            version: 1,
            seq,
            at_ms: 1_793_023_327_120,
            event: Event { window, terminal, body },
        })
    }

    /// The time column depends on the zone: what follows it is what is checked.
    fn after_time(text: Option<String>) -> String {
        let text = text.expect("a line");
        assert!(text.ends_with('\n'));
        text[12..].trim_end().to_string()
    }

    #[test]
    fn the_header_counts_what_there_is_to_watch() {
        let terminal = |id| crate::ipc::control::TerminalInfo {
            id,
            name: "t".into(),
            status: TerminalState::Idle,
            agent: None,
            folder: None,
            branch: None,
            changed: None,
            pinned: false,
            profile: None,
            selected: false,
            tabs: vec![],
        };
        let subscribed = Subscribed {
            subscribed: true,
            seq: 430,
            boot: "x".into(),
            topics: vec![],
            listing: Listing {
                windows: vec![
                    WindowInfo { id: 1, front: true, terminals: vec![terminal(1), terminal(2)] },
                    WindowInfo { id: 2, front: false, terminals: vec![terminal(3)] },
                ],
            },
        };
        assert_eq!(header(&subscribed), "watching 2 windows, 3 terminals from #430 (ctrl-c to stop)\n");
        let one = Subscribed {
            listing: Listing {
                windows: vec![WindowInfo { id: 1, front: true, terminals: vec![terminal(1)] }],
            },
            ..subscribed
        };
        assert!(header(&one).starts_with("watching 1 window, 1 terminal from"));
    }

    #[test]
    fn events_read_as_aligned_lines() {
        let home = Some(Path::new("/Users/me"));
        let status = event(
            431,
            Some(1),
            Some(3),
            Body::Status {
                status: TerminalState::NeedsYou,
                was: TerminalState::Working,
                agent: Some("claude".into()),
                message: Some("Allow Bash?".into()),
            },
        );
        assert_eq!(
            after_time(line(&status, home, PLAIN)),
            "  #431    w1 t3     status    working → needs you  claude  \"Allow Bash?\""
        );
        let finished = event(
            432,
            Some(1),
            Some(8),
            Body::Block {
                block: 57,
                phase: BlockPhase::Finished,
                exit_code: Some(101),
                took_ms: Some(3512),
                command: Some("cargo test".into()),
            },
        );
        assert_eq!(
            after_time(line(&finished, home, PLAIN)),
            "  #432    w1 t8     block     finished 101  3.5s  cargo test"
        );
        let started = event(
            433,
            Some(1),
            Some(8),
            Body::Block {
                block: 58,
                phase: BlockPhase::Started,
                exit_code: None,
                took_ms: None,
                command: None,
            },
        );
        assert_eq!(after_time(line(&started, home, PLAIN)), "  #433    w1 t8     block     started  #58");
        let split =
            event(434, Some(1), None, Body::Split { split: 12, terminals: vec![3, 9], focused: Some(9) });
        assert_eq!(
            after_time(line(&split, home, PLAIN)),
            "  #434    w1        split     12: t3 t9  focus t9"
        );
        let opened =
            event(435, Some(2), Some(10), Body::TerminalOpened { folder: Some("/Users/me/src/api".into()) });
        assert_eq!(after_time(line(&opened, home, PLAIN)), "  #435    w2 t10    terminal  opened  ~/src/api");
        let window = event(436, Some(2), None, Body::WindowClosed);
        assert_eq!(after_time(line(&window, home, PLAIN)), "  #436    w2        window    closed");
        let bell = event(437, Some(1), Some(3), Body::Bell);
        assert_eq!(after_time(line(&bell, home, PLAIN)), "  #437    w1 t3     bell");
    }

    #[test]
    fn what_a_terminal_said_cannot_drive_the_terminal_it_is_printed_in() {
        let hostile = event(
            1,
            Some(1),
            Some(1),
            Body::Title { title: Some("\u{1b}]0;pwned\u{7}\u{202e}evil".into()), name: "x\u{1b}[2J".into() },
        );
        let text = line(&hostile, None, PLAIN).expect("a line");
        assert!(
            !text.contains('\u{1b}') && !text.contains('\u{7}') && !text.contains('\u{202e}'),
            "{text:?}"
        );
        assert!(text.contains("\\u{1b}]0;pwned\\u{7}"), "{text}");
        let command = event(
            2,
            Some(1),
            Some(1),
            Body::Block {
                block: 1,
                phase: BlockPhase::Started,
                exit_code: None,
                took_ms: None,
                command: Some("echo \u{1b}[31m".into()),
            },
        );
        assert!(!line(&command, None, PLAIN).expect("a line").contains('\u{1b}'));
    }

    #[test]
    fn colour_only_when_asked_and_only_on_states_and_codes() {
        let status = event(
            1,
            Some(1),
            Some(3),
            Body::Status {
                status: TerminalState::Failed,
                was: TerminalState::Working,
                agent: None,
                message: None,
            },
        );
        let colored = line(&status, None, Style { color: true }).expect("a line");
        assert!(
            colored.contains("\x1b[31mfailed\x1b[0m") && colored.contains("\x1b[34mworking\x1b[0m"),
            "{colored:?}"
        );
        assert!(!line(&status, None, PLAIN).expect("a line").contains('\x1b'));
    }

    #[test]
    fn the_streams_own_lines_and_their_exit_codes() {
        let heartbeat = StreamLine::Notice(Notice::Heartbeat { seq: 3 });
        assert_eq!(line(&heartbeat, None, PLAIN), None, "a heartbeat is silent in text");
        let lagged = StreamLine::Notice(Notice::Lagged { dropped: 12 });
        assert_eq!(line(&lagged, None, PLAIN).as_deref(), Some("-- fell behind: 12 events were dropped\n"));
        let closed = StreamLine::Notice(Notice::Closed { reason: CloseReason::Quitting });
        assert_eq!(line(&closed, None, PLAIN).as_deref(), Some("-- closed: Wormhole is quitting\n"));
        assert_eq!(exit_code(CloseReason::Quitting), 0);
        assert_eq!(exit_code(CloseReason::NotAllowed), 1);
        assert_eq!(exit_code(CloseReason::TooSlow), 1);
    }
}
