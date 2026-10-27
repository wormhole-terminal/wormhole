//! Typing and clicking: `send_text`, `send_keys` and `mouse`. All three need `control_write` and never go
//! into the caller's own terminal or one that is waiting for the person: the app decides, as for `wh`.

use serde::Deserialize;
use serde_json::{Value, json};

use super::schema::{boolean, field, input, integer, one_of, props, string, terminal};
use super::terminals::reached_output;
use super::{Call, Hints, Tool, read};
use crate::ipc::control::mouse::{ButtonName, CLICK_COUNT_MAX, Direction, Pointer, SCROLL_LINES_MAX, Wheel};
use crate::ipc::control::{
    Action, KeyItem, MouseOp, MouseSpec, SEND_TEXT_MAX_BYTES, SendKeysSpec, Target,
    keys::{KEY_SPEC_MAX_BYTES, KEYS_MAX_ITEMS},
};
use crate::ipc::mcp::exec::Endpoint;

pub(super) const TOOLS: &[Tool] = &[
    Tool {
        name: "send_text",
        title: "Type text into a terminal",
        description: "Paste text into a terminal as the person would (wrapped in bracketed-paste markers when the \
            program asked for them); with `submit` it then presses Return. Use it to type a command or a prompt \
            into a shell or another agent. To press special keys (ctrl-c, arrows, escape) use send_keys. To run \
            a command and get its output and exit code use `run`. Needs Settings > Coding agents > \"Let the \
            command line type and open terminals\" (control_write). Refused for your own terminal and for a \
            terminal that is waiting for the person's answer (status needs_you).",
        hints: Hints::DRIVES,
        long: false,
        flag: false,
        input: text_input,
        output: reached_output,
        build: build_text,
    },
    Tool {
        name: "send_keys",
        title: "Press keys in a terminal",
        description: "Press keys in a terminal, in order, encoded for what the program has asked for (application \
            cursor keys, the kitty keyboard protocol): ctrl-c to interrupt, escape, enter, tab, shift-tab, \
            up/down/left/right, home/end, pageup/pagedown, f1..f24, alt-x, a single character. A key is a \
            string such as \"ctrl-c\"; {\"text\": \"ls\"} types a string key by key as typing would (printable \
            characters only), {\"text\": \"...\", \"paste\": true} pastes it. A key that makes the program \
            change its modes should end the call. Use this to drive TUIs, REPLs and other agents' UIs; read \
            the result with read_terminal. Needs control_write; same refusals as send_text.",
        hints: Hints::DRIVES,
        long: false,
        flag: true,
        input: keys_input,
        output: keys_output,
        build: build_keys,
    },
    Tool {
        name: "mouse",
        title: "Click or scroll in a terminal",
        description: "Click, press, release, move (with `button` set: drag) or scroll the wheel at a cell of a \
            terminal, for a program that asked for the mouse (reported by read_terminal as modes.mouse). \
            Rows and columns count from 0 at the screen's top left. A terminal whose program did not ask for \
            the mouse refuses (no_mouse): Wormhole does not emulate selection. Needs control_write; same \
            refusals as send_text.",
        hints: Hints::DRIVES,
        long: false,
        flag: true,
        input: mouse_input,
        output: reached_output,
        build: build_mouse,
    },
];

// send_text

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextArgs {
    terminal: i64,
    text: String,
    #[serde(default)]
    submit: bool,
}

fn text_input() -> Value {
    input(
        props(&[
            ("terminal", terminal("The terminal's id from list_terminals.")),
            ("text", string("What to type (at most 16384 bytes).", SEND_TEXT_MAX_BYTES)),
            ("submit", boolean("Press Return after the text.")),
        ]),
        &["terminal", "text"],
    )
}

fn build_text(arguments: Value, _: &Endpoint) -> Result<Call, String> {
    let args: TextArgs = read("send_text", arguments)?;
    Ok(Call::Control(Action::SendText {
        target: Target::Terminal(args.terminal),
        text: args.text,
        submit: args.submit,
    }))
}

// send_keys

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct KeysArgs {
    terminal: i64,
    keys: Vec<KeyArg>,
}

/// A key as a bare string, or as the wire has it.
#[derive(Deserialize)]
#[serde(untagged)]
enum KeyArg {
    Name(String),
    Item(KeyItem),
}

fn keys_input() -> Value {
    input(
        props(&[
            ("terminal", terminal("The terminal's id from list_terminals.")),
            (
                "keys",
                json!({
                    "type": "array", "minItems": 1, "maxItems": KEYS_MAX_ITEMS,
                    "description": "What to press, in order.",
                    "items": {"oneOf": [
                        string("A key: ctrl-c, enter, escape, tab, shift-tab, up, f5, alt-x, ctrl-d, a.", KEY_SPEC_MAX_BYTES),
                        input(props(&[("key", string("A key, as above.", KEY_SPEC_MAX_BYTES))]), &["key"]),
                        input(props(&[
                            ("text", string("Printable text, at most 4096 bytes in all.", crate::ipc::control::keys::KEYS_TEXT_MAX_BYTES)),
                            ("paste", boolean("Paste it (bracketed when the program asked) instead of typing it key by key; a paste may hold newlines.")),
                        ]), &["text"]),
                    ]}
                }),
            ),
        ]),
        &["terminal", "keys"],
    )
}

fn keys_output() -> Value {
    super::schema::output(
        props(&[
            ("tab", field("integer", "The tab it reached.")),
            ("keys", field("integer", "How many items were sent.")),
        ]),
        &["tab", "keys"],
    )
}

fn build_keys(arguments: Value, _: &Endpoint) -> Result<Call, String> {
    let args: KeysArgs = read("send_keys", arguments)?;
    let keys = args
        .keys
        .into_iter()
        .map(|key| match key {
            KeyArg::Name(key) => KeyItem::Key { key },
            KeyArg::Item(item) => item,
        })
        .collect();
    Ok(Call::Control(Action::SendKeys(SendKeysSpec { target: Target::Terminal(args.terminal), keys })))
}

// mouse

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum MouseAction {
    Click,
    Press,
    Release,
    Move,
    Scroll,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MouseArgs {
    terminal: i64,
    action: MouseAction,
    row: u32,
    col: u32,
    button: Option<ButtonName>,
    count: Option<u8>,
    direction: Option<Direction>,
    lines: Option<u16>,
    #[serde(default)]
    shift: bool,
    #[serde(default)]
    alt: bool,
    #[serde(default)]
    ctrl: bool,
}

fn mouse_input() -> Value {
    input(
        props(&[
            ("terminal", terminal("The terminal's id from list_terminals.")),
            (
                "action",
                one_of(
                    "What the pointer does: click (press and release), press, release, move, scroll.",
                    &["click", "press", "release", "move", "scroll"],
                ),
            ),
            ("row", integer("0-based row of the screen (inside the grid).", 0, 100_000)),
            ("col", integer("0-based column.", 0, 100_000)),
            (
                "button",
                one_of(
                    "The button for click, press, release; with move, the button held down (a drag). Default: left (none for move).",
                    &["left", "middle", "right"],
                ),
            ),
            (
                "count",
                integer("Clicks in a row for click (2 is a double click).", 1, i64::from(CLICK_COUNT_MAX)),
            ),
            (
                "direction",
                one_of("For scroll (required): which way the wheel turns.", &["up", "down", "left", "right"]),
            ),
            ("lines", integer("For scroll: how many lines (default 1).", 1, i64::from(SCROLL_LINES_MAX))),
            ("shift", boolean("Hold shift.")),
            ("alt", boolean("Hold alt.")),
            ("ctrl", boolean("Hold ctrl.")),
        ]),
        &["terminal", "action", "row", "col"],
    )
}

fn build_mouse(arguments: Value, _: &Endpoint) -> Result<Call, String> {
    let args: MouseArgs = read("mouse", arguments)?;
    let pointer = || Pointer {
        row: args.row,
        col: args.col,
        button: args.button,
        count: args.count.unwrap_or(1),
        shift: args.shift,
        alt: args.alt,
        ctrl: args.ctrl,
    };
    let op = match args.action {
        MouseAction::Click => MouseOp::Click(pointer()),
        MouseAction::Press => MouseOp::Press(pointer()),
        MouseAction::Release => MouseOp::Release(pointer()),
        MouseAction::Move => MouseOp::Move(pointer()),
        MouseAction::Scroll => {
            let direction =
                args.direction.ok_or("mouse scroll needs a direction: up, down, left or right")?;
            MouseOp::Scroll(Wheel {
                row: args.row,
                col: args.col,
                direction,
                lines: args.lines.unwrap_or(1),
                shift: args.shift,
                alt: args.alt,
                ctrl: args.ctrl,
            })
        }
    };
    Ok(Call::Control(Action::Mouse(MouseSpec { target: Target::Terminal(args.terminal), op })))
}
