//! Arranging: `new_terminal`, `new_window`, `split`, `focus`, `rename` and `close`. All need
//! `control_write`. None types into a terminal, so naming your own terminal is fine (to split it for a dev
//! server, to name it); `close` only ever asks the person.

use serde::Deserialize;
use serde_json::Value;

use super::schema::{boolean, field, input, integer, one_of, output, props, string, terminal};
use super::{Call, Hints, Tool, read};
use crate::ipc::control::structure::{COMMAND_MAX_BYTES, NAME_MAX_CHARS};
use crate::ipc::control::{
    Action, CloseSpec, FocusSpec, NewWindowSpec, RenameSpec, Side, SplitSpec, Target, resolve_folder,
};
use crate::ipc::mcp::exec::Endpoint;

pub(super) const TOOLS: &[Tool] = &[
    Tool {
        name: "new_terminal",
        title: "Open a terminal",
        description: "Open a new terminal (a new sidebar row) in a Wormhole window, in a folder, optionally \
            running a command in its shell. Returns its window and terminal ids; the command is typed into \
            the shell, so use `wait` or `run` to follow it. With `worktree` it asks the person to create a \
            git worktree on that branch first and the terminal opens once they confirm. Needs the person's permission to type and open.",
        hints: Hints::DRIVES,
        long: false,
        flag: false,
        input: new_terminal_input,
        output: new_terminal_output,
        build: build_new_terminal,
    },
    Tool {
        name: "new_window",
        title: "Open a window",
        description: "Open a new Wormhole window with one terminal, in a folder, optionally running a command. It \
            opens behind your other windows and takes no keyboard focus unless `activate`. Returns the window \
            and terminal ids. Needs the person's permission to type and open.",
        hints: Hints::DRIVES,
        long: false,
        flag: true,
        input: new_window_input,
        output: window_opened_output,
        build: build_new_window,
    },
    Tool {
        name: "split",
        title: "Split a terminal",
        description: "Open a new terminal beside a terminal, shown together with it in a split: in its folder and \
            shell unless `folder` is given, optionally running a command (for example a dev server next to \
            yourself). The selection and the keyboard stay where they are unless `select`. Nothing changes \
            when the window has no room (refused, no_room). Returns the new terminal's id. Needs the person's permission to type and open.",
        hints: Hints::DRIVES,
        long: false,
        flag: true,
        input: split_input,
        output: split_output,
        build: build_split,
    },
    Tool {
        name: "focus",
        title: "Focus a terminal",
        description: "Select a terminal in its window's sidebar and show its split. The keyboard stays where it \
            was, and no other app is interrupted, unless `activate`, which also gives the terminal the keys \
            and brings its window forward. Needs the person's permission to type and open.",
        hints: Hints::ARRANGES,
        long: false,
        flag: true,
        input: focus_input,
        output: focused_output,
        build: build_focus,
    },
    Tool {
        name: "rename",
        title: "Rename a terminal",
        description: "Name a terminal as the sidebar's rename does (one line, cut to 64 characters). An empty or \
            missing `name` gives the terminal its automatic name back. Needs the person's permission to type and open.",
        hints: Hints::ARRANGES,
        long: false,
        flag: true,
        input: rename_input,
        output: renamed_output,
        build: build_rename,
    },
    Tool {
        name: "close",
        title: "Ask to close a terminal",
        description: "Ask the person whether to close a terminal. Nothing closes on your say: the result only \
            says that the question is up in the terminal's window, and it does not tell the answer (the \
            terminal disappearing from list_terminals does). Refused while a close question is already up in \
            that window (question_open). Needs the person's permission to type and open.",
        hints: Hints { read_only: false, destructive: true, idempotent: false, open_world: false },
        long: false,
        flag: true,
        input: close_input,
        output: close_output,
        build: build_close,
    },
];

fn folder_property() -> Value {
    string(
        "A folder: absolute, ~/…, or relative to the working directory of this MCP server. Default: the folder the \
         terminal it follows is in.",
        4096,
    )
}

fn command_property() -> Value {
    string(
        "A command line typed into the new terminal's shell (one line, at most 16384 bytes).",
        COMMAND_MAX_BYTES,
    )
}

fn folder_of(folder: Option<String>, at: &Endpoint) -> Option<String> {
    folder.map(|folder| resolve_folder(&folder, &at.cwd, at.home.as_deref()))
}

// new_terminal

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewTerminalArgs {
    folder: Option<String>,
    command: Option<String>,
    worktree: Option<String>,
    window: Option<i64>,
}

fn new_terminal_input() -> Value {
    input(
        props(&[
            ("folder", folder_property()),
            ("command", command_property()),
            (
                "worktree",
                string("Create a git worktree on this branch for the terminal; asks the person first.", 256),
            ),
            ("window", terminal("The window's id from list_terminals. Default: the front window.")),
        ]),
        &[],
    )
}

fn new_terminal_output() -> Value {
    output(
        props(&[
            ("window", field("integer", "The window it opened in.")),
            ("terminal", field("integer", "The new terminal's id.")),
            ("tab", field("integer", "Its tab.")),
            ("pane", field("integer", "Its pane.")),
            (
                "asked",
                field(
                    "boolean",
                    "With worktree: the person is being asked; the terminal opens once they confirm.",
                ),
            ),
        ]),
        &[],
    )
}

fn build_new_terminal(arguments: Value, at: &Endpoint) -> Result<Call, String> {
    let args: NewTerminalArgs = read("new_terminal", arguments)?;
    Ok(Call::Control(Action::NewTerminal {
        folder: folder_of(args.folder, at),
        command: args.command,
        worktree: args.worktree,
        window: args.window,
    }))
}

// new_window

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewWindowArgs {
    folder: Option<String>,
    command: Option<String>,
    #[serde(default)]
    activate: bool,
}

fn new_window_input() -> Value {
    input(
        props(&[
            ("folder", folder_property()),
            ("command", command_property()),
            (
                "activate",
                boolean(
                    "Bring the window forward and give it the keyboard (it otherwise opens behind other apps).",
                ),
            ),
        ]),
        &[],
    )
}

fn window_opened_output() -> Value {
    output(
        props(&[
            ("window", field("integer", "The new window's id.")),
            ("terminal", field("integer", "Its terminal's id.")),
        ]),
        &["window", "terminal"],
    )
}

fn build_new_window(arguments: Value, at: &Endpoint) -> Result<Call, String> {
    let args: NewWindowArgs = read("new_window", arguments)?;
    Ok(Call::Control(Action::NewWindow(NewWindowSpec {
        folder: folder_of(args.folder, at),
        command: args.command,
        activate: args.activate,
    })))
}

// split

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SplitArgs {
    terminal: i64,
    direction: Option<Side>,
    size: Option<u8>,
    folder: Option<String>,
    command: Option<String>,
    #[serde(default)]
    select: bool,
}

fn split_input() -> Value {
    input(
        props(&[
            ("terminal", terminal("The terminal to split beside, from list_terminals.")),
            (
                "direction",
                one_of(
                    "Which side of it the new terminal goes (default right).",
                    &["right", "down", "left", "up"],
                ),
            ),
            (
                "size",
                integer(
                    "The new terminal's share of the split, in percent (held by the room there is).",
                    1,
                    99,
                ),
            ),
            ("folder", folder_property()),
            ("command", command_property()),
            ("select", boolean("Select the new terminal and give it the window's keyboard.")),
        ]),
        &["terminal"],
    )
}

fn split_output() -> Value {
    output(
        props(&[
            ("window", field("integer", "The window.")),
            ("split", field("integer", "The split's id.")),
            ("terminal", field("integer", "The new terminal's id.")),
            (
                "size",
                field("integer", "The share the new terminal got, when size was given and could be applied."),
            ),
        ]),
        &["window", "split", "terminal"],
    )
}

fn build_split(arguments: Value, at: &Endpoint) -> Result<Call, String> {
    let args: SplitArgs = read("split", arguments)?;
    Ok(Call::Control(Action::Split(SplitSpec {
        target: Target::Terminal(args.terminal),
        direction: args.direction.unwrap_or_default(),
        size: args.size,
        folder: folder_of(args.folder, at),
        command: args.command,
        select: args.select,
    })))
}

// focus

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FocusArgs {
    terminal: i64,
    #[serde(default)]
    activate: bool,
}

fn focus_input() -> Value {
    input(
        props(&[
            ("terminal", terminal("The terminal's id from list_terminals.")),
            ("activate", boolean("Also give it the keyboard and bring its window forward.")),
        ]),
        &["terminal"],
    )
}

fn focused_output() -> Value {
    output(
        props(&[
            ("window", field("integer", "Its window.")),
            ("terminal", field("integer", "The terminal selected.")),
        ]),
        &["window", "terminal"],
    )
}

fn build_focus(arguments: Value, _: &Endpoint) -> Result<Call, String> {
    let args: FocusArgs = read("focus", arguments)?;
    Ok(Call::Control(Action::Focus(FocusSpec {
        target: Target::Terminal(args.terminal),
        activate: args.activate,
    })))
}

// rename

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RenameArgs {
    terminal: i64,
    name: Option<String>,
}

fn rename_input() -> Value {
    input(
        props(&[
            ("terminal", terminal("The terminal's id from list_terminals.")),
            (
                "name",
                string(
                    "The new name (at most 64 characters). Empty or missing: the automatic name.",
                    NAME_MAX_CHARS,
                ),
            ),
        ]),
        &["terminal"],
    )
}

fn renamed_output() -> Value {
    output(
        props(&[
            ("terminal", field("integer", "The terminal.")),
            ("name", field("string", "The name the sidebar shows now.")),
            ("custom", field("boolean", "Whether the name was given (not automatic).")),
        ]),
        &["terminal", "name", "custom"],
    )
}

fn build_rename(arguments: Value, _: &Endpoint) -> Result<Call, String> {
    let args: RenameArgs = read("rename", arguments)?;
    Ok(Call::Control(Action::Rename(RenameSpec { target: Target::Terminal(args.terminal), name: args.name })))
}

// close

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CloseArgs {
    terminal: i64,
}

fn close_input() -> Value {
    input(props(&[("terminal", terminal("The terminal's id from list_terminals."))]), &["terminal"])
}

fn close_output() -> Value {
    output(
        props(&[
            ("asked", field("boolean", "The question is up; the person has not answered.")),
            ("window", field("integer", "The window it is up in.")),
            ("terminal", field("integer", "The terminal.")),
        ]),
        &["asked", "window", "terminal"],
    )
}

fn build_close(arguments: Value, _: &Endpoint) -> Result<Call, String> {
    let args: CloseArgs = read("close", arguments)?;
    Ok(Call::Control(Action::Close(CloseSpec { target: Target::Terminal(args.terminal) })))
}
