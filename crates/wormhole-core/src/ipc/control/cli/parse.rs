//! The arguments, read into the request they ask for.

use std::path::{Path, PathBuf};

use super::{Cli, Parsed};
use crate::ipc::control::keys::{KeyItem, SendKeysSpec, parse_key};
use crate::ipc::control::mouse::{ButtonName, Direction, MouseOp, MouseSpec, Pointer, Wheel};
use crate::ipc::control::read::{BlockRef, BlocksSpec, Newest, Range, ReadSpec};
use crate::ipc::control::{
    Action, CloseSpec, EventsSpec, FocusSpec, NewWindowSpec, RenameSpec, RunSpec, Side, SplitSpec, Target,
    TerminalState, WaitSpec,
};

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
    let mut text_out = false;
    let mut target = None;
    let mut title = None;
    let mut submit = false;
    let mut folder = None;
    let mut command = None;
    let mut worktree = None;
    let mut window = None;
    let mut activate = false;
    let mut select = false;
    let mut clear = false;
    let mut size = None;
    let mut side: Option<Side> = None;
    let mut watch = EventsSpec::default();
    let mut look = Look::default();
    let mut items: Vec<KeyItem> = Vec::new();
    let mut pointer = PointerFlags::default();
    let mut waiting = WaitFlags::default();
    let mut exit_code = false;
    let mut words: Vec<&str> = Vec::new();
    let mut args = rest.iter().map(String::as_str);
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| args.next().ok_or_else(|| format!("{flag} needs a value"));
        let id = |flag: &str, value: &str| {
            value.parse::<i64>().map_err(|_| format!("{flag} takes an id (a number), not {value:?}"))
        };
        match arg {
            "--json" => json = true,
            // On `wait`, `--text` is the pattern to find (the plain-text format is `WORMHOLE_JSON=0` there).
            "--text" if verb == "wait" => waiting.text = Some(value(arg)?.to_string()),
            "--text" => text_out = true,
            "--help" | "-h" => return Ok(Parsed::Help),
            // `events` watches several terminals and windows: its `--terminal` and `--window` repeat.
            "--terminal" if verb == "events" => {
                let value = value(arg)?;
                watch.terminals.push(id(arg, value)?);
            }
            "--window" if verb == "events" => {
                let value = value(arg)?;
                watch.windows.push(id(arg, value)?);
            }
            "--topics" if verb == "events" => {
                let value = value(arg)?;
                watch
                    .topics
                    .extend(value.split(',').map(str::trim).filter(|t| !t.is_empty()).map(String::from));
            }
            "--output" if verb == "events" => watch.output = true,
            "--output" if verb == "blocks" => look.output = true,
            "--last" if verb == "blocks" => {
                let value = value(arg)?;
                look.last = Some(count(arg, value)?);
            }
            "--join" if verb == "read" => look.join = true,
            "--styles" if verb == "read" => look.styles = true,
            "--numbers" if verb == "read" => look.numbers = true,
            "--last" if verb == "read" => {
                let value = value(arg)?;
                look.last_rows = Some(count(arg, value)?);
            }
            "--lines" if verb == "read" => {
                let value = value(arg)?;
                look.lines = Some(rows(value)?);
            }
            "--block" if verb == "read" => {
                let value = value(arg)?;
                look.block = Some(match value {
                    "last" => BlockRef::Newest(Newest::Last),
                    id => BlockRef::Id(id.parse().map_err(|_| {
                        format!("--block takes a block's id (a number) or `last`, not {id:?}")
                    })?),
                });
            }
            "--status" if verb == "wait" => {
                let value = value(arg)?;
                for word in value.split(',').map(str::trim).filter(|word| !word.is_empty()) {
                    waiting.status.push(state(word)?);
                }
            }
            "--gone" if verb == "wait" => waiting.gone = Some(value(arg)?.to_string()),
            "--stable" if verb == "wait" => {
                let value = value(arg)?;
                waiting.stable_ms = Some(
                    value
                        .trim_end_matches("ms")
                        .parse()
                        .map_err(|_| format!("--stable takes milliseconds, not {value:?}"))?,
                );
            }
            "--block-done" if verb == "wait" => waiting.block_done = true,
            "--timeout" if matches!(verb, "wait" | "run") => waiting.timeout_ms = Some(seconds(value(arg)?)?),
            "--exit-code" if verb == "run" => exit_code = true,
            // Everything after `--` is the command, whatever it starts with.
            "--" if verb == "run" => {
                words.extend(args.by_ref());
                break;
            }
            "--type" if verb == "send-keys" => {
                items.push(KeyItem::Text { text: value(arg)?.to_string(), paste: false });
            }
            "--paste" if verb == "send-keys" => {
                items.push(KeyItem::Text { text: value(arg)?.to_string(), paste: true });
            }
            "--button" if verb == "mouse" => {
                pointer.button = Some(match value(arg)? {
                    "left" => ButtonName::Left,
                    "middle" => ButtonName::Middle,
                    "right" => ButtonName::Right,
                    other => return Err(format!("--button takes left, middle or right, not {other:?}")),
                });
            }
            "--double" if verb == "mouse" => pointer.count = 2,
            "--shift" if verb == "mouse" => pointer.shift = true,
            "--alt" if verb == "mouse" => pointer.alt = true,
            "--ctrl" if verb == "mouse" => pointer.ctrl = true,
            "--by" if verb == "mouse" => {
                let value = value(arg)?;
                pointer.lines = u16::try_from(count(arg, value)?)
                    .map_err(|_| format!("--by takes a smaller number, not {value:?}"))?;
            }
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
            "--folder" if opens(verb) => folder = Some(value(arg)?.to_string()),
            "--command" if opens(verb) => command = Some(value(arg)?.to_string()),
            "--activate" if matches!(verb, "new-window" | "focus") => activate = true,
            "--select" if verb == "split" => select = true,
            "--clear" if verb == "rename" => clear = true,
            "--size" if verb == "split" => {
                let value = value(arg)?;
                size = Some(
                    value
                        .trim_end_matches('%')
                        .parse::<u8>()
                        .map_err(|_| format!("--size takes a percent (1 to 99), not {value:?}"))?,
                );
            }
            "--right" | "--down" | "--left" | "--up" if verb == "split" => {
                let one = match arg {
                    "--right" => Side::Right,
                    "--down" => Side::Down,
                    "--left" => Side::Left,
                    _ => Side::Up,
                };
                if side.replace(one).is_some() {
                    return Err("split goes one way: --right, --down, --left or --up".into());
                }
            }
            "--worktree" if verb == "new-terminal" => worktree = Some(value(arg)?.to_string()),
            "--window" if verb == "new-terminal" => {
                let value = value(arg)?;
                window = Some(id(arg, value)?);
            }
            // A key's name (`-` is one) among send-keys' words, in the order it was typed with the texts.
            word if verb == "send-keys" && (!word.starts_with('-') || word == "-") => {
                parse_key(word).map_err(|e| format!("send-keys: {e}"))?;
                items.push(KeyItem::Key { key: word.to_string() });
            }
            flag if flag.starts_with('-') && flag != "-" => {
                return Err(format!("unknown option {flag} for {verb}"));
            }
            word => words.push(word),
        }
    }
    if json && text_out {
        return Err("--json and --text can't be combined".into());
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
        "new-window" => {
            if !text.is_empty() {
                return Err(format!("new-window takes options only, not {text:?} (did you mean --folder?)"));
            }
            let folder = folder.map(|folder| resolve_folder(&folder, cwd, home));
            Action::NewWindow(NewWindowSpec { folder, command, activate })
        }
        "split" => {
            let target = target.take().ok_or_else(|| needs_target("split"))?;
            if !text.is_empty() {
                return Err(format!("split takes options only, not {text:?}"));
            }
            let folder = folder.map(|folder| resolve_folder(&folder, cwd, home));
            Action::Split(SplitSpec {
                target,
                direction: side.unwrap_or_default(),
                size,
                folder,
                command,
                select,
            })
        }
        "focus" => {
            let target = target.take().ok_or_else(|| needs_target("focus"))?;
            if !text.is_empty() {
                return Err(format!("focus takes options only, not {text:?}"));
            }
            Action::Focus(FocusSpec { target, activate })
        }
        "rename" => {
            let target = target.take().ok_or_else(|| needs_target("rename"))?;
            match (clear, text.trim().is_empty()) {
                (true, false) => return Err("rename takes a name or --clear, not both".into()),
                (false, true) => return Err("rename needs a name (or --clear)".into()),
                _ => {}
            }
            Action::Rename(RenameSpec { target, name: (!clear).then_some(text) })
        }
        "close" => {
            let target = target.take().ok_or_else(|| needs_target("close"))?;
            if !text.is_empty() {
                return Err(format!("close takes a target only, not {text:?}"));
            }
            Action::Close(CloseSpec { target })
        }
        "events" => {
            if !text.is_empty() {
                return Err(format!("events takes options only, not {text:?}"));
            }
            Action::Events(watch)
        }
        "read" => {
            if !text.is_empty() {
                return Err(format!("read takes options only, not {text:?}"));
            }
            if look.numbers && look.join {
                return Err("--numbers counts rows: it can't be combined with --join".into());
            }
            Action::Read(ReadSpec {
                target: target.take(),
                range: look.range()?,
                join: look.join,
                styles: look.styles,
            })
        }
        "blocks" => {
            if !text.is_empty() {
                return Err(format!("blocks takes options only, not {text:?}"));
            }
            Action::Blocks(BlocksSpec { target: target.take(), last: look.last, output: look.output })
        }
        "send-keys" => {
            let Some(target) = target.take() else {
                return Err("send-keys needs a target: --terminal, --tab or --pane".into());
            };
            if items.is_empty() {
                return Err("send-keys needs keys (ctrl-c, enter, up, f5…), --type or --paste".into());
            }
            Action::SendKeys(SendKeysSpec { target, keys: items })
        }
        "wait" => {
            let target = target.take().ok_or_else(|| needs_target("wait"))?;
            if !text.is_empty() {
                return Err(format!("wait takes options only, not {text:?}"));
            }
            let WaitFlags { status, text, gone, stable_ms, block_done, timeout_ms } = waiting;
            Action::Wait(WaitSpec { target, status, text, gone, stable_ms, block_done, timeout_ms })
        }
        "run" => {
            let target = target.take().ok_or_else(|| needs_target("run"))?;
            if text.trim().is_empty() {
                return Err("run needs the command to run (after the options, in quotes or after --)".into());
            }
            Action::Run(RunSpec { target, command: text, timeout_ms: waiting.timeout_ms })
        }
        "mouse" => {
            let Some(target) = target.take() else {
                return Err("mouse needs a target: --terminal, --tab or --pane".into());
            };
            Action::Mouse(MouseSpec { target, op: pointer.op(&words)? })
        }
        other => return Err(format!("unknown command {other:?}")),
    };
    if target.is_some()
        && matches!(
            action,
            Action::List | Action::NewTerminal { .. } | Action::NewWindow(_) | Action::Events(_)
        )
    {
        return Err(format!("{verb} takes no target"));
    }
    let stdin = matches!(&action, Action::SendText { text, .. } if text == "-");
    Ok(Parsed::Run(Cli { action, json, text: text_out, stdin, numbers: look.numbers, exit_code }))
}

/// What a verb that names its target says when it has none.
fn needs_target(verb: &str) -> String {
    format!("{verb} needs a target: --terminal, --tab or --pane")
}

/// Whether the verb starts a terminal, and so takes the folder and the command it starts in.
fn opens(verb: &str) -> bool {
    matches!(verb, "new-terminal" | "new-window" | "split")
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

/// What `wait` (and `run`'s timeout) were asked for, as the flags gave it.
#[derive(Default)]
struct WaitFlags {
    status: Vec<TerminalState>,
    text: Option<String>,
    gone: Option<String>,
    stable_ms: Option<u64>,
    block_done: bool,
    timeout_ms: Option<u64>,
}

/// A state as `wait --status` spells it (`needs_you`, also `needs-you`).
fn state(word: &str) -> Result<TerminalState, String> {
    match word.replace('-', "_").as_str() {
        "idle" => Ok(TerminalState::Idle),
        "working" => Ok(TerminalState::Working),
        "needs_you" => Ok(TerminalState::NeedsYou),
        "finished" => Ok(TerminalState::Finished),
        "failed" => Ok(TerminalState::Failed),
        "exited" => Ok(TerminalState::Exited),
        _ => {
            Err(format!("--status takes idle, working, needs_you, finished, failed or exited, not {word:?}"))
        }
    }
}

/// `--timeout`'s seconds (fractions allowed) as milliseconds.
fn seconds(value: &str) -> Result<u64, String> {
    let mistake = || format!("--timeout takes seconds, not {value:?}");
    let secs: f64 = value.trim_end_matches('s').parse().map_err(|_| mistake())?;
    if !secs.is_finite() || secs <= 0.0 || secs > 1e9 {
        return Err(mistake());
    }
    Ok((secs * 1000.0).round().max(1.0) as u64)
}

/// What `read` and `blocks` were asked for, as the flags gave it.
#[derive(Default)]
struct Look {
    /// `read --last <n>`: the last n rows.
    last_rows: Option<u32>,
    lines: Option<(i32, i32)>,
    block: Option<BlockRef>,
    join: bool,
    styles: bool,
    numbers: bool,
    last: Option<u32>,
    output: bool,
}

impl Look {
    /// The range the flags name: at most one of them, the screen without any.
    fn range(&self) -> Result<Range, String> {
        match (self.last_rows, self.lines, self.block) {
            (None, None, None) => Ok(Range::Screen),
            (Some(lines), None, None) => Ok(Range::Last { lines }),
            (None, Some((from, to)), None) => Ok(Range::Lines { from, to }),
            (None, None, Some(id)) => Ok(Range::Block { id }),
            _ => Err("read takes one of --last, --lines and --block".into()),
        }
    }
}

/// The pointer's flags.
struct PointerFlags {
    button: Option<ButtonName>,
    count: u8,
    shift: bool,
    alt: bool,
    ctrl: bool,
    lines: u16,
}

impl Default for PointerFlags {
    fn default() -> Self {
        Self { button: None, count: 1, shift: false, alt: false, ctrl: false, lines: 1 }
    }
}

impl PointerFlags {
    /// `click <row> <col>`, `press|release|move <row> <col>` or `scroll <row> <col> <direction>`.
    fn op(&self, words: &[&str]) -> Result<MouseOp, String> {
        let at = |word: Option<&&str>, what: &str| {
            let word = word.ok_or_else(|| format!("mouse needs a {what}"))?;
            word.parse::<u32>().map_err(|_| format!("the {what} is a number (from 0), not {word:?}"))
        };
        let usage = "mouse takes click, press, release, move or scroll, then a row and a column";
        let kind = *words.first().ok_or(usage)?;
        let (row, col) = (at(words.get(1), "row")?, at(words.get(2), "column")?);
        // A click, press or release is of the left button unless told; a move has a button down only
        // when told (a drag).
        let pointer = |button| Pointer {
            row,
            col,
            button,
            count: self.count,
            shift: self.shift,
            alt: self.alt,
            ctrl: self.ctrl,
        };
        let op = match kind {
            "click" => MouseOp::Click(pointer(Some(self.button.unwrap_or(ButtonName::Left)))),
            "press" => MouseOp::Press(pointer(Some(self.button.unwrap_or(ButtonName::Left)))),
            "release" => MouseOp::Release(pointer(Some(self.button.unwrap_or(ButtonName::Left)))),
            "move" => MouseOp::Move(pointer(self.button)),
            "scroll" => {
                let direction = match words.get(3).copied() {
                    Some("up") => Direction::Up,
                    Some("down") => Direction::Down,
                    Some("left") => Direction::Left,
                    Some("right") => Direction::Right,
                    _ => return Err("scroll takes a direction: up, down, left or right".into()),
                };
                MouseOp::Scroll(Wheel {
                    row,
                    col,
                    direction,
                    lines: self.lines,
                    shift: self.shift,
                    alt: self.alt,
                    ctrl: self.ctrl,
                })
            }
            _ => return Err(usage.into()),
        };
        let expected = if kind == "scroll" { 4 } else { 3 };
        if words.len() > expected {
            return Err(format!("mouse {kind} takes {} words, not {}", expected - 1, words.len() - 1));
        }
        Ok(op)
    }
}

/// A count flag's value.
fn count(flag: &str, value: &str) -> Result<u32, String> {
    value.parse().map_err(|_| format!("{flag} takes a number, not {value:?}"))
}

/// `--lines <from>:<to>`: grid lines, history negative.
fn rows(value: &str) -> Result<(i32, i32), String> {
    let mistake = || format!("--lines takes <from>:<to> (grid lines, history negative), not {value:?}");
    let (from, to) = value.split_once(':').ok_or_else(mistake)?;
    Ok((from.trim().parse().map_err(|_| mistake())?, to.trim().parse().map_err(|_| mistake())?))
}
