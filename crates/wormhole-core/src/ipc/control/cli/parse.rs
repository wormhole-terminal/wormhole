//! The arguments, read into the request they ask for.

use std::path::{Path, PathBuf};

use super::{Cli, Parsed};
use crate::ipc::control::keys::{KeyItem, SendKeysSpec, parse_key};
use crate::ipc::control::mouse::{ButtonName, Direction, MouseOp, MouseSpec, Pointer, Wheel};
use crate::ipc::control::read::{BlockRef, BlocksSpec, Newest, Range, ReadSpec};
use crate::ipc::control::{Action, EventsSpec, Target};

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
    let mut watch = EventsSpec::default();
    let mut look = Look::default();
    let mut items: Vec<KeyItem> = Vec::new();
    let mut pointer = PointerFlags::default();
    let mut words: Vec<&str> = Vec::new();
    let mut args = rest.iter().map(String::as_str);
    while let Some(arg) = args.next() {
        let mut value = |flag: &str| args.next().ok_or_else(|| format!("{flag} needs a value"));
        let id = |flag: &str, value: &str| {
            value.parse::<i64>().map_err(|_| format!("{flag} takes an id (a number), not {value:?}"))
        };
        match arg {
            "--json" => json = true,
            // send-keys' own `--text <string>` is a segment of text, not the output format.
            "--text" if verb == "send-keys" => {
                items.push(KeyItem::Text { text: value(arg)?.to_string(), paste: false });
            }
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
            "--scrollback" if verb == "read" => {
                let value = value(arg)?;
                look.scrollback = Some(count(arg, value)?);
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
            "--paste" if verb == "send-keys" => {
                items.push(KeyItem::Text { text: value(arg)?.to_string(), paste: true });
            }
            "--button" if verb == "mouse" => {
                pointer.button = match value(arg)? {
                    "left" => ButtonName::Left,
                    "middle" => ButtonName::Middle,
                    "right" => ButtonName::Right,
                    other => return Err(format!("--button takes left, middle or right, not {other:?}")),
                };
            }
            "--double" if verb == "mouse" => pointer.count = 2,
            "--shift" if verb == "mouse" => pointer.shift = true,
            "--alt" if verb == "mouse" => pointer.alt = true,
            "--ctrl" if verb == "mouse" => pointer.ctrl = true,
            "--lines" if verb == "mouse" => {
                let value = value(arg)?;
                pointer.lines = u16::try_from(count(arg, value)?)
                    .map_err(|_| format!("--lines takes a smaller number, not {value:?}"))?;
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
            "--folder" if verb == "new-terminal" => folder = Some(value(arg)?.to_string()),
            "--command" if verb == "new-terminal" => command = Some(value(arg)?.to_string()),
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
                return Err("send-keys needs keys (ctrl-c, enter, up, f5…) or --text".into());
            }
            Action::SendKeys(SendKeysSpec { target, keys: items })
        }
        "mouse" => {
            let Some(target) = target.take() else {
                return Err("mouse needs a target: --terminal, --tab or --pane".into());
            };
            Action::Mouse(MouseSpec { target, op: pointer.op(&words)? })
        }
        other => return Err(format!("unknown command {other:?}")),
    };
    if target.is_some() && matches!(action, Action::List | Action::NewTerminal { .. } | Action::Events(_)) {
        return Err(format!("{verb} takes no target"));
    }
    let stdin = matches!(&action, Action::SendText { text, .. } if text == "-");
    Ok(Parsed::Run(Cli { action, json, text: text_out, stdin, numbers: look.numbers }))
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

/// What `read` and `blocks` were asked for, as the flags gave it.
#[derive(Default)]
struct Look {
    scrollback: Option<u32>,
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
        match (self.scrollback, self.lines, self.block) {
            (None, None, None) => Ok(Range::Screen),
            (Some(lines), None, None) => Ok(Range::Last { lines }),
            (None, Some((from, to)), None) => Ok(Range::Lines { from, to }),
            (None, None, Some(id)) => Ok(Range::Block { id }),
            _ => Err("read takes one of --scrollback, --lines and --block".into()),
        }
    }
}

/// The pointer's flags.
struct PointerFlags {
    button: ButtonName,
    count: u8,
    shift: bool,
    alt: bool,
    ctrl: bool,
    lines: u16,
}

impl Default for PointerFlags {
    fn default() -> Self {
        Self { button: ButtonName::Left, count: 1, shift: false, alt: false, ctrl: false, lines: 1 }
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
        let pointer = || Pointer {
            row,
            col,
            button: self.button,
            count: self.count,
            shift: self.shift,
            alt: self.alt,
            ctrl: self.ctrl,
        };
        let op = match kind {
            "click" => MouseOp::Click(pointer()),
            "press" => MouseOp::Press(pointer()),
            "release" => MouseOp::Release(pointer()),
            "move" => MouseOp::Move(pointer()),
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Cli, Parsed, parse};
    use crate::ipc::control::{Action, EventsSpec, Target};

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
        assert_eq!(
            parsed(&["list"]),
            Cli { action: Action::List, json: false, text: false, stdin: false, numbers: false }
        );
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
    fn events_takes_topics_terminals_windows_and_output() {
        assert_eq!(parsed(&["events"]).action, Action::Events(EventsSpec::default()));
        assert_eq!(
            parsed(&[
                "events",
                "--topics",
                "status, blocks",
                "--topics",
                "bell",
                "--terminal",
                "3",
                "--terminal",
                "4",
                "--window",
                "1",
                "--output",
                "--json"
            ]),
            Cli {
                action: Action::Events(EventsSpec {
                    topics: vec!["status".into(), "blocks".into(), "bell".into()],
                    terminals: vec![3, 4],
                    windows: vec![1],
                    output: true,
                }),
                json: true,
                text: false,
                stdin: false,
                numbers: false,
            }
        );
        assert!(mistake(&["events", "--tab", "3"]).contains("takes no target"), "no tabs here");
        assert!(mistake(&["events", "--terminal", "x"]).contains("takes an id"));
        assert!(mistake(&["events", "status"]).contains("options only"));
        assert!(mistake(&["events", "--topics"]).contains("needs a value"));
        assert!(mistake(&["list", "--topics", "x"]).contains("unknown option"));
    }

    #[test]
    fn read_and_blocks_take_a_range_and_options() {
        use crate::ipc::control::read::{BlockRef, BlocksSpec, Newest, Range, ReadSpec};
        let read = |list: &[&str]| match parsed(list).action {
            Action::Read(spec) => spec,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            read(&["read"]),
            ReadSpec { target: None, range: Range::Screen, join: false, styles: false }
        );
        let spec = read(&["read", "--terminal", "3", "--scrollback", "200", "--join"]);
        assert_eq!(spec.target, Some(Target::Terminal(3)));
        assert_eq!((spec.range, spec.join), (Range::Last { lines: 200 }, true));
        assert_eq!(
            read(&["read", "--lines", "-200:39", "--styles"]).range,
            Range::Lines { from: -200, to: 39 }
        );
        assert_eq!(read(&["read", "--block", "57"]).range, Range::Block { id: BlockRef::Id(57) });
        assert_eq!(
            read(&["read", "--block", "last"]).range,
            Range::Block { id: BlockRef::Newest(Newest::Last) }
        );
        assert!(parsed(&["read", "--numbers"]).numbers && !parsed(&["read"]).numbers);
        assert!(mistake(&["read", "--scrollback", "5", "--block", "last"]).contains("one of"));
        assert!(mistake(&["read", "--lines", "5"]).contains("<from>:<to>"));
        assert!(mistake(&["read", "--block", "first"]).contains("block's id"));
        assert!(mistake(&["read", "now"]).contains("options only"));
        assert_eq!(
            parsed(&["blocks", "--tab", "2", "--last", "5", "--output"]).action,
            Action::Blocks(BlocksSpec { target: Some(Target::Tab(2)), last: Some(5), output: true })
        );
        assert!(mistake(&["blocks", "--join"]).contains("unknown option"));
    }

    #[test]
    fn send_keys_keeps_keys_and_texts_in_the_order_they_were_typed() {
        use crate::ipc::control::keys::{KeyItem, SendKeysSpec};
        let keys = |text: &str| KeyItem::Text { text: text.into(), paste: false };
        assert_eq!(
            parsed(&["send-keys", "--terminal", "4", "ctrl-c", "--text", "ls -la", "enter", "--paste", "x"])
                .action,
            Action::SendKeys(SendKeysSpec {
                target: Target::Terminal(4),
                keys: vec![
                    KeyItem::Key { key: "ctrl-c".into() },
                    keys("ls -la"),
                    KeyItem::Key { key: "enter".into() },
                    KeyItem::Text { text: "x".into(), paste: true },
                ],
            })
        );
        let dash = parsed(&["send-keys", "--tab", "1", "-", "ctrl--"]);
        assert!(matches!(&dash.action, Action::SendKeys(spec) if spec.keys.len() == 2));
        assert!(mistake(&["send-keys", "ctrl-c"]).contains("needs a target"));
        assert!(mistake(&["send-keys", "--tab", "1"]).contains("needs keys"));
        assert!(mistake(&["send-keys", "--tab", "1", "cmd-q"]).contains("Wormhole's own key"));
        assert!(mistake(&["send-keys", "--tab", "1", "nope"]).contains("unknown key"));
        assert!(mistake(&["send-keys", "--tab", "1", "--bogus"]).contains("unknown option"));
        assert!(mistake(&["send-keys", "--tab", "1", "--text"]).contains("needs a value"));
    }

    #[test]
    fn mouse_takes_an_operation_a_place_and_the_keys_held() {
        use crate::ipc::control::mouse::{ButtonName, Direction, MouseOp, MouseSpec, Pointer, Wheel};
        let op = |list: &[&str]| match parsed(list).action {
            Action::Mouse(MouseSpec { target, op }) => (target, op),
            other => panic!("{other:?}"),
        };
        let pointer =
            |button, count, shift| Pointer { row: 4, col: 10, button, count, shift, alt: false, ctrl: false };
        assert_eq!(
            op(&["mouse", "--terminal", "3", "click", "4", "10"]),
            (Target::Terminal(3), MouseOp::Click(pointer(ButtonName::Left, 1, false)))
        );
        assert_eq!(
            op(&["mouse", "--terminal", "3", "click", "4", "10", "--double", "--button", "right", "--shift"])
                .1,
            MouseOp::Click(pointer(ButtonName::Right, 2, true))
        );
        assert_eq!(
            op(&["mouse", "--tab", "3", "press", "4", "10"]).1,
            MouseOp::Press(pointer(ButtonName::Left, 1, false))
        );
        assert_eq!(
            op(&["mouse", "--tab", "3", "release", "4", "10"]).1,
            MouseOp::Release(pointer(ButtonName::Left, 1, false))
        );
        assert_eq!(
            op(&["mouse", "--tab", "3", "move", "4", "10"]).1,
            MouseOp::Move(pointer(ButtonName::Left, 1, false))
        );
        assert_eq!(
            op(&["mouse", "--tab", "3", "scroll", "4", "10", "down", "--lines", "3", "--ctrl"]).1,
            MouseOp::Scroll(Wheel {
                row: 4,
                col: 10,
                direction: Direction::Down,
                lines: 3,
                shift: false,
                alt: false,
                ctrl: true
            })
        );
        assert!(mistake(&["mouse", "click", "1", "2"]).contains("needs a target"));
        assert!(mistake(&["mouse", "--tab", "1"]).contains("click, press"));
        assert!(mistake(&["mouse", "--tab", "1", "click", "x", "2"]).contains("row"));
        assert!(mistake(&["mouse", "--tab", "1", "click", "1"]).contains("column"));
        assert!(mistake(&["mouse", "--tab", "1", "scroll", "1", "2"]).contains("direction"));
        assert!(mistake(&["mouse", "--tab", "1", "click", "1", "2", "3"]).contains("takes 2 words"));
        assert!(
            mistake(&["mouse", "--tab", "1", "click", "1", "2", "--button", "back"])
                .contains("left, middle or right")
        );
        assert!(mistake(&["mouse", "--tab", "1", "wiggle", "1", "2"]).contains("click, press"));
    }

    #[test]
    fn the_output_format_is_chosen_by_a_flag() {
        assert!(parsed(&["list", "--text"]).text);
        assert!(!parsed(&["list"]).text);
        assert!(mistake(&["list", "--json", "--text"]).contains("can't be combined"));
    }

    #[test]
    fn help_is_asked_for_in_every_usual_way() {
        for list in [&[][..], &["help"], &["--help"], &["-h"], &["list", "--help"], &["send-text", "-h"]] {
            assert_eq!(parse(&words(list), Path::new("/"), None).expect("parses"), Parsed::Help, "{list:?}");
        }
        let help = crate::ipc::control::help();
        assert!(
            help.contains(&format!("{} list", crate::ipc::control::COMMAND_NAME)) && help.contains("--json")
        );
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
}
