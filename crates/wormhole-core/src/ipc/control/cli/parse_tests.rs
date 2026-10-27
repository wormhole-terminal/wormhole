use std::path::Path;

use super::{Cli, Parsed, parse};
use crate::ipc::control::{
    Action, CloseSpec, EventsSpec, FocusSpec, NewWindowSpec, RenameSpec, Side, SplitSpec, Target,
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
    assert_eq!(
        parsed(&["list"]),
        Cli {
            action: Action::List,
            json: false,
            text: false,
            stdin: false,
            numbers: false,
            exit_code: false
        }
    );
    assert!(parsed(&["list", "--json"]).json);
    assert_eq!(parsed(&["status"]).action, Action::Status { target: None });
    assert_eq!(
        parsed(&["status", "--terminal", "3"]).action,
        Action::Status { target: Some(Target::Terminal(3)) }
    );
    assert_eq!(
        parsed(&["notify", "--tab", "9", "--title", "Deploy", "done", "in", "3s"]).action,
        Action::Notify { target: Some(Target::Tab(9)), title: "Deploy".into(), body: "done in 3s".into() }
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
        Action::NewTerminal { folder: Some("/Users/me".into()), command: None, worktree: None, window: None }
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
            exit_code: false,
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
    assert_eq!(read(&["read"]), ReadSpec { target: None, range: Range::Screen, join: false, styles: false });
    let spec = read(&["read", "--terminal", "3", "--last", "200", "--join"]);
    assert_eq!(spec.target, Some(Target::Terminal(3)));
    assert_eq!((spec.range, spec.join), (Range::Last { lines: 200 }, true));
    assert_eq!(read(&["read", "--lines", "-200:39", "--styles"]).range, Range::Lines { from: -200, to: 39 });
    assert_eq!(read(&["read", "--block", "57"]).range, Range::Block { id: BlockRef::Id(57) });
    assert_eq!(read(&["read", "--block", "last"]).range, Range::Block { id: BlockRef::Newest(Newest::Last) });
    assert!(parsed(&["read", "--numbers"]).numbers && !parsed(&["read"]).numbers);
    assert!(mistake(&["read", "--last", "5", "--block", "last"]).contains("one of"));
    assert!(mistake(&["read", "--numbers", "--join"]).contains("--join"));
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
        parsed(&["send-keys", "--terminal", "4", "ctrl-c", "--type", "ls -la", "enter", "--paste", "x"])
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
    assert!(mistake(&["send-keys", "--tab", "1", "--type"]).contains("needs a value"));
    // `--text` is the output format here as everywhere.
    let formatted = parsed(&["send-keys", "--tab", "1", "enter", "--text"]);
    assert!(formatted.text && matches!(&formatted.action, Action::SendKeys(spec) if spec.keys.len() == 1));
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
        (Target::Terminal(3), MouseOp::Click(pointer(Some(ButtonName::Left), 1, false)))
    );
    assert_eq!(
        op(&["mouse", "--terminal", "3", "click", "4", "10", "--double", "--button", "right", "--shift"]).1,
        MouseOp::Click(pointer(Some(ButtonName::Right), 2, true))
    );
    assert_eq!(
        op(&["mouse", "--tab", "3", "press", "4", "10"]).1,
        MouseOp::Press(pointer(Some(ButtonName::Left), 1, false))
    );
    assert_eq!(
        op(&["mouse", "--tab", "3", "release", "4", "10"]).1,
        MouseOp::Release(pointer(Some(ButtonName::Left), 1, false))
    );
    assert_eq!(
        op(&["mouse", "--tab", "3", "move", "4", "10"]).1,
        MouseOp::Move(pointer(None, 1, false)),
        "a move holds no button unless told"
    );
    assert_eq!(
        op(&["mouse", "--tab", "3", "scroll", "4", "10", "down", "--by", "3", "--ctrl"]).1,
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
    assert!(help.contains(&format!("{} list", crate::ipc::control::COMMAND_NAME)) && help.contains("--json"));
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

#[test]
fn the_window_and_split_verbs_parse_with_their_options() {
    assert_eq!(
        parsed(&["new-window"]).action,
        Action::NewWindow(NewWindowSpec { folder: None, command: None, activate: false })
    );
    assert_eq!(
        parsed(&["new-window", "--folder", "~/src/app", "--command", "claude", "--activate"]).action,
        Action::NewWindow(NewWindowSpec {
            folder: Some("/Users/me/src/app".into()),
            command: Some("claude".into()),
            activate: true,
        })
    );
    // The right unless told, as ⌘D splits; the size takes a percent sign too.
    let split = |direction, size, select| {
        Action::Split(SplitSpec {
            target: Target::Terminal(3),
            direction,
            size,
            folder: None,
            command: None,
            select,
        })
    };
    assert_eq!(parsed(&["split", "--terminal", "3"]).action, split(Side::Right, None, false));
    assert_eq!(
        parsed(&["split", "--terminal", "3", "--down", "--size", "40%", "--select"]).action,
        split(Side::Down, Some(40), true)
    );
    for (flag, side) in [("--right", Side::Right), ("--left", Side::Left), ("--up", Side::Up)] {
        assert_eq!(parsed(&["split", "--terminal", "3", flag]).action, split(side, None, false), "{flag}");
    }
    assert_eq!(
        parsed(&["split", "--tab", "3", "--folder", "src", "--command", "npm run dev"]).action,
        Action::Split(SplitSpec {
            target: Target::Tab(3),
            direction: Side::Right,
            size: None,
            folder: Some("/work/here/src".into()),
            command: Some("npm run dev".into()),
            select: false,
        })
    );
    assert_eq!(
        parsed(&["focus", "--terminal", "3", "--activate"]).action,
        Action::Focus(FocusSpec { target: Target::Terminal(3), activate: true })
    );
    assert_eq!(
        parsed(&["rename", "--terminal", "3", "dev", "server"]).action,
        Action::Rename(RenameSpec { target: Target::Terminal(3), name: Some("dev server".into()) })
    );
    assert_eq!(
        parsed(&["rename", "--terminal", "3", "--clear"]).action,
        Action::Rename(RenameSpec { target: Target::Terminal(3), name: None })
    );
    assert_eq!(
        parsed(&["close", "--terminal", "3", "--json"]).action,
        Action::Close(CloseSpec { target: Target::Terminal(3) })
    );
}

#[test]
fn the_window_and_split_verbs_name_their_mistakes() {
    for verb in ["split", "focus", "rename", "close"] {
        assert!(mistake(&[verb]).contains("needs a target"), "{verb}");
    }
    assert!(mistake(&["new-window", "--terminal", "3"]).contains("takes no target"));
    assert!(mistake(&["new-window", "/work/app"]).contains("--folder"));
    assert!(mistake(&["split", "--terminal", "3", "--right", "--down"]).contains("one way"));
    assert!(mistake(&["split", "--terminal", "3", "--size", "big"]).contains("a percent"));
    assert!(mistake(&["split", "--terminal", "3", "--size", "300"]).contains("a percent"));
    assert!(mistake(&["split", "--terminal", "3", "stray"]).contains("options only"));
    assert!(mistake(&["rename", "--terminal", "3"]).contains("needs a name"));
    assert!(mistake(&["rename", "--terminal", "3", "--clear", "api"]).contains("not both"));
    assert!(mistake(&["close", "--terminal", "3", "now"]).contains("target only"));
    // An option of another verb is no option of these.
    assert!(mistake(&["focus", "--terminal", "3", "--select"]).contains("unknown option"));
    assert!(mistake(&["close", "--terminal", "3", "--activate"]).contains("unknown option"));
    assert!(mistake(&["new-window", "--select"]).contains("unknown option"));
}

#[test]
fn wait_takes_conditions_in_any_mix() {
    use crate::ipc::control::{RunSpec, TerminalState, WaitSpec};

    let wait = |args: &[&str]| match parsed(args).action {
        Action::Wait(spec) => spec,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        wait(&[
            "wait",
            "--terminal",
            "3",
            "--status",
            "idle,needs-you",
            "--status",
            "failed",
            "--text",
            r"\$ $",
            "--gone",
            "Loading",
            "--stable",
            "800",
            "--block-done",
            "--timeout",
            "45"
        ]),
        WaitSpec {
            target: Target::Terminal(3),
            status: vec![TerminalState::Idle, TerminalState::NeedsYou, TerminalState::Failed],
            text: Some(r"\$ $".into()),
            gone: Some("Loading".into()),
            stable_ms: Some(800),
            block_done: true,
            timeout_ms: Some(45_000),
        }
    );
    // Fractions of a second, and units said out loud.
    assert_eq!(wait(&["wait", "--tab", "1", "--block-done", "--timeout", "0.5s"]).timeout_ms, Some(500));
    assert_eq!(wait(&["wait", "--tab", "1", "--stable", "250ms"]).stable_ms, Some(250));
    // `--text` is the pattern on `wait`; on every other verb it is the output format.
    assert!(!parsed(&["wait", "--terminal", "3", "--text", "x"]).text);
    assert!(parsed(&["list", "--text"]).text);
    // The command line's own exit-code option and the command's words.
    let run = parsed(&["run", "--terminal", "3", "--timeout", "90", "--exit-code", "cargo test"]);
    assert!(run.exit_code);
    assert_eq!(
        run.action,
        Action::Run(RunSpec {
            target: Target::Terminal(3),
            command: "cargo test".into(),
            timeout_ms: Some(90_000)
        })
    );
    let after_dashes = parsed(&["run", "--terminal", "3", "--", "ls", "-la", "--color=never"]);
    assert!(!after_dashes.exit_code);
    assert_eq!(
        after_dashes.action,
        Action::Run(RunSpec {
            target: Target::Terminal(3),
            command: "ls -la --color=never".into(),
            timeout_ms: None
        })
    );
}

#[test]
fn wait_and_run_name_their_mistakes() {
    assert!(mistake(&["wait"]).contains("needs a target"));
    assert!(mistake(&["run"]).contains("needs a target"));
    assert!(mistake(&["run", "--terminal", "3"]).contains("needs the command"));
    assert!(mistake(&["wait", "--terminal", "3", "stray"]).contains("options only"));
    assert!(mistake(&["wait", "--terminal", "3", "--status", "asleep"]).contains("--status takes"));
    assert!(mistake(&["wait", "--terminal", "3", "--stable", "soon"]).contains("milliseconds"));
    for bad in ["soon", "0", "-5", "nan"] {
        assert!(mistake(&["wait", "--terminal", "3", "--timeout", bad]).contains("seconds"), "{bad}");
    }
    assert!(mistake(&["wait", "--terminal", "3", "--exit-code"]).contains("unknown option"));
    assert!(mistake(&["list", "--timeout", "3"]).contains("unknown option"));
    assert!(mistake(&["wait", "--terminal", "3", "--text"]).contains("needs a value"));
}
