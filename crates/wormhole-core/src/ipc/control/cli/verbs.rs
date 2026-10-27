//! The command line's verbs as data: each with its arguments, its flags (name, value, meaning) and
//! what it needs to be allowed. `is_verb` reads this table, `wh help --json` prints it, and the tests
//! in `guard_tests` hold it to the parser (`parse`) and to the skill that teaches agents the command
//! line, so the list of what exists is written once.
//!
//! The usage text (`help`) is prose for people and is not generated from here; what is checked is that
//! every flag the parser reads is in this table and the reverse.

/// What a verb or a flag needs before the app answers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ipc) enum Need {
    /// The Automation feature flag (Settings → Feature flags).
    Automation,
    /// Settings → Coding agents → "Let the command line type and open terminals" (`control_write`).
    Write,
    /// Settings → Coding agents → "Let the command line read terminals" (`control_read`); a terminal
    /// can always read itself.
    Read,
}

impl Need {
    /// The word `help --json` and the docs use for it.
    pub(in crate::ipc) fn word(self) -> &'static str {
        match self {
            Need::Automation => "automation_flag",
            Need::Write => "control_write",
            Need::Read => "control_read",
        }
    }
}

/// One option of a verb.
#[derive(Debug, Clone, Copy)]
pub(in crate::ipc) struct Flag {
    pub(in crate::ipc) name: &'static str,
    /// What it takes after it, in angle brackets, if anything.
    pub(in crate::ipc) value: Option<&'static str>,
    /// May be given more than once.
    pub(in crate::ipc) repeats: bool,
    pub(in crate::ipc) meaning: &'static str,
    /// What it needs beyond what its verb needs.
    pub(in crate::ipc) needs: Option<Need>,
}

const fn flag(name: &'static str, value: Option<&'static str>, meaning: &'static str) -> Flag {
    Flag { name, value, repeats: false, meaning, needs: None }
}

impl Flag {
    const fn repeating(mut self) -> Self {
        self.repeats = true;
        self
    }

    const fn needing(mut self, need: Need) -> Self {
        self.needs = Some(need);
        self
    }
}

/// How a verb names the terminal it is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ipc) enum Target {
    /// It is not about one terminal.
    No,
    /// `--terminal`, `--tab` or `--pane`; inside a terminal the caller's own without one.
    Optional,
    Required,
}

/// A verb.
#[derive(Debug, Clone, Copy)]
pub(in crate::ipc) struct Verb {
    pub(in crate::ipc) name: &'static str,
    /// Its arguments after the name, as usage prints them.
    pub(in crate::ipc) args: &'static str,
    pub(in crate::ipc) does: &'static str,
    pub(in crate::ipc) target: Target,
    pub(in crate::ipc) needs: &'static [Need],
    pub(in crate::ipc) flags: &'static [Flag],
    /// Words that come right after the verb, for verbs that have subcommands.
    pub(in crate::ipc) subcommands: &'static [&'static str],
    /// Works without the app running.
    pub(in crate::ipc) offline: bool,
}

const fn verb(name: &'static str, args: &'static str, does: &'static str) -> Verb {
    Verb { name, args, does, target: Target::No, needs: &[], flags: &[], subcommands: &[], offline: false }
}

impl Verb {
    const fn about(mut self, target: Target) -> Self {
        self.target = target;
        self
    }

    const fn needing(mut self, needs: &'static [Need]) -> Self {
        self.needs = needs;
        self
    }

    const fn with(mut self, flags: &'static [Flag]) -> Self {
        self.flags = flags;
        self
    }

    const fn then(mut self, subcommands: &'static [&'static str]) -> Self {
        self.subcommands = subcommands;
        self
    }

    const fn offline(mut self) -> Self {
        self.offline = true;
        self
    }
}

/// Flags every verb that goes through the parser takes.
pub(in crate::ipc) const GLOBAL_FLAGS: &[Flag] = &[
    flag("--json", None, "print the reply as one JSON object (WORMHOLE_JSON=1 makes that the default)"),
    flag("--text", None, "print text even when WORMHOLE_JSON is set"),
    flag("--help", None, "print the usage"),
];

/// The flags that name the terminal a verb is about.
pub(in crate::ipc) const TARGET_FLAGS: &[Flag] = &[
    flag("--terminal", Some("<id>"), "the terminal, by the id `list` prints"),
    flag("--tab", Some("<id>"), "the same terminal (a terminal has one tab); either id works"),
    flag("--pane", Some("<id>"), "the same terminal (a terminal has one pane); either id works"),
];

use Need::{Automation, Read, Write};

const FOLDER: Flag =
    flag("--folder", Some("<path>"), "the folder it starts in (~ and relative paths are resolved here)");
const COMMAND: Flag = flag("--command", Some("<command>"), "a command its shell runs at start");

/// Every verb, in the order the docs give them.
pub(in crate::ipc) const VERBS: &[Verb] = &[
    verb("list", "", "every window, terminal, tab and pane with their ids"),
    verb("status", "[<target>]", "one terminal's status (this terminal's without a target), or a summary of the app")
        .about(Target::Optional),
    verb("notify", "[<target>] [--title <text>] <body>...", "a notification about a terminal, in your words")
        .about(Target::Optional)
        .with(&[flag("--title", Some("<text>"), "the notification's title")]),
    verb("send-text", "<target> [--submit] <text>... | -", "type text into a terminal as a paste; - reads stdin")
        .about(Target::Required)
        .needing(&[Write])
        .with(&[flag("--submit", None, "press Return after the text")]),
    verb(
        "new-terminal",
        "[--folder <path>] [--command <command>] [--window <id>] [--worktree <branch>]",
        "a new terminal in the front window (or --window)",
    )
    .needing(&[Write])
    .with(&[
        FOLDER,
        COMMAND,
        flag("--window", Some("<id>"), "the window to open it in"),
        flag("--worktree", Some("<branch>"), "ask the user to confirm a new git worktree on this branch"),
    ]),
    verb(
        "events",
        "[--topics <list>] [--terminal <id>]... [--window <id>]... [--output]",
        "watch what happens, as it happens (one line each; --json gives one JSON object per line)",
    )
    .needing(&[Automation])
    .with(&[
        flag("--topics", Some("<list>"), "comma-separated: windows, terminals, status, title, folder, bell, blocks, splits, output"),
        flag("--terminal", Some("<id>"), "only this terminal's events").repeating(),
        flag("--window", Some("<id>"), "only this window's events").repeating(),
        flag("--output", None, "also what terminals print").needing(Read),
    ]),
    verb(
        "read",
        "[<target>] [--last <n> | --lines <from>:<to> | --block <id>|last] [--join] [--styles] [--numbers]",
        "what a terminal shows: its screen, history or one command block, as text",
    )
    .about(Target::Optional)
    .needing(&[Automation, Read])
    .with(&[
        flag("--last", Some("<n>"), "the last n rows of history and screen"),
        flag("--lines", Some("<from>:<to>"), "grid lines: 0 is the screen's top row, history is negative"),
        flag("--block", Some("<id>|last"), "one command block: prompt, command and output"),
        flag("--join", None, "put wrapped rows back together"),
        flag("--styles", None, "with --json, each line's colours and attributes"),
        flag("--numbers", None, "a gutter of line numbers in the text output"),
    ]),
    verb("blocks", "[<target>] [--last <n>] [--output]", "a terminal's commands: id, state, exit code, time")
        .about(Target::Optional)
        .needing(&[Automation, Read])
        .with(&[
            flag("--last", Some("<n>"), "the newest n commands (at most 100, 10 by default)"),
            flag("--output", None, "what each command printed"),
        ]),
    verb(
        "send-keys",
        "<target> <key>... [--type <string>]... [--paste <string>]...",
        "press keys in a terminal, encoded for the program's keyboard modes",
    )
    .about(Target::Required)
    .needing(&[Automation, Write])
    .with(&[
        flag("--type", Some("<string>"), "type a string key by key").repeating(),
        flag("--paste", Some("<string>"), "paste a string (in the program's paste brackets)").repeating(),
    ]),
    verb(
        "mouse",
        "<target> click|press|release|move <row> <col> | scroll <row> <col> up|down|left|right",
        "click, drag or scroll in a program that asked for the mouse (rows and columns count from 0)",
    )
    .about(Target::Required)
    .needing(&[Automation, Write])
    .with(&[
        flag("--button", Some("left|middle|right"), "the button (left for a click, press and release)"),
        flag("--double", None, "a double click"),
        flag("--by", Some("<n>"), "wheel lines for scroll"),
        flag("--shift", None, "with shift held"),
        flag("--alt", None, "with alt held"),
        flag("--ctrl", None, "with ctrl held"),
    ]),
    verb("new-window", "[--folder <path>] [--command <command>] [--activate]", "a window with one new terminal, opened behind the user's other windows")
        .needing(&[Automation, Write])
        .with(&[FOLDER, COMMAND, flag("--activate", None, "open it in front and give it the keys")]),
    verb(
        "split",
        "<target> [--right|--down|--left|--up] [--size <percent>] [--folder <path>] [--command <command>] [--select]",
        "a new terminal beside the target, in its split",
    )
    .about(Target::Required)
    .needing(&[Automation, Write])
    .with(&[
        flag("--right", None, "to the right of the target (the default)"),
        flag("--down", None, "below the target"),
        flag("--left", None, "to the left of the target"),
        flag("--up", None, "above the target"),
        flag("--size", Some("<percent>"), "its share of the split, 1 to 99"),
        FOLDER,
        COMMAND,
        flag("--select", None, "select the new terminal and give it the keys"),
    ]),
    verb("focus", "<target> [--activate]", "select a terminal and show its split; the keys stay where they were")
        .about(Target::Required)
        .needing(&[Automation, Write])
        .with(&[flag("--activate", None, "give the terminal the keys and bring its window forward")]),
    verb("rename", "<target> <name>... | --clear", "name a terminal, as the sidebar's rename does")
        .about(Target::Required)
        .needing(&[Automation, Write])
        .with(&[flag("--clear", None, "call it after what runs in it again")]),
    verb("close", "<target>", "ask the user, in the terminal's window, whether to close it; it never closes by itself")
        .about(Target::Required)
        .needing(&[Automation, Write]),
    verb(
        "wait",
        "<target> [--status <state>[,<state>]...] [--match <regex>] [--gone <regex>] [--stable <ms>] [--block-done [--after <block>]] [--timeout <s>]",
        "wait until one of the conditions holds and say which; exits 124 when the time is up",
    )
    .about(Target::Required)
    .needing(&[Automation])
    .with(&[
        flag("--status", Some("<state>[,<state>]"), "idle, working, needs_you, finished, failed or exited"),
        flag("--match", Some("<regex>"), "a pattern found on the screen").needing(Read),
        flag("--gone", Some("<regex>"), "a pattern no longer found on the screen").needing(Read),
        flag("--stable", Some("<ms>"), "no output for this long"),
        flag("--block-done", None, "a command finishes (needs shell integration)"),
        flag("--after", Some("<block>"), "with --block-done: only a command after this block counts"),
        flag("--timeout", Some("<s>"), "seconds to wait: 30 by default, at most 600"),
    ]),
    verb(
        "run",
        "<target> [--timeout <s>] [--exit-code] ['<command>' | -- <word>...]",
        "type a command at the shell's prompt, wait for it and give its output and exit code",
    )
    .about(Target::Required)
    .needing(&[Automation, Write])
    .with(&[
        flag("--timeout", Some("<s>"), "seconds to wait: 10 minutes by default, at most 3600"),
        flag("--exit-code", None, "exit with the command's own exit code"),
        flag("--", None, "everything after it is the command's words"),
    ]),
    verb("doctor", "", "what an agent needs to know in one call: the app, the switches, this terminal, the skill, what to fix"),
    verb("skill", "install|uninstall|show [--agent <id>] [--path] [<file>]", "install the Wormhole skill into coding agents' skill folders, remove it, or show it")
        .then(&["install", "uninstall", "show"])
        .with(&[
            flag("--agent", Some("<id>"), "one agent (claude, codex, gemini, opencode, qwen, copilot, droid); every installed one without"),
            flag("--path", None, "show: print where the skill goes instead of the skill"),
        ])
        .offline(),
    verb("sessions", "list|end <id>|end --all|forget <id>", "the terminals that keep running behind the app")
        .then(&["list", "end", "forget"])
        .with(&[flag("--all", None, "end: every background terminal of this data folder, not one")])
        .offline(),
    verb("mcp", "", "serve all of this to a coding agent over MCP on stdin and stdout").offline(),
    verb("welcome", "[-- <program> [<args>...]]", "the first-run welcome, in this terminal").offline(),
    verb("help", "[--json]", "the usage; --json the verbs, flags, exit codes and error codes as data").offline(),
];

/// The verb called `name`.
pub(in crate::ipc) fn find(name: &str) -> Option<&'static Verb> {
    VERBS.iter().find(|verb| verb.name == name)
}

/// What the command line exits with, as data.
pub(in crate::ipc) const EXIT_CODES: &[(i32, &str)] = &[
    (0, "it worked"),
    (
        1,
        "the app would not or could not (see error.code: not_found, not_allowed, refused, rate_limited, too_large, unsupported, failed)",
    ),
    (2, "a mistake in the arguments (bad_request), or a protocol version the app does not speak"),
    (3, "Wormhole is not running, or did not answer (an older build, or one still starting)"),
    (124, "a wait or run ran out of time (error.reason is timeout)"),
];
