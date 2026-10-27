# Commands

Every verb of `wh`, with the flags worth knowing. `wh help --json` prints all of it as data, from the
parser's own tables; use that when this page and the program disagree. A target is
`--terminal <id>` (`--tab` and `--pane` name the same terminal); ids come from `wh list --json`.
Every verb takes `--json`. Needs: **flag** is the Automation flag, **write** the user's `control_write`
switch, **read** their `control_read` switch (never needed for your own terminal).

## Looking

- `wh doctor` — the state of everything an agent depends on, with fixes. Needs nothing. Run it first.
- `wh list` — windows, terminals, tabs, panes and splits with ids. Needs nothing.
- `wh status [<target>]` — one terminal's status (`idle`, `working`, `needs_you`, `finished`, `failed`,
  `exited`), agent, folder and branch; with no target inside a terminal, your own; outside, a summary.
- `wh notify [<target>] [--title <text>] <body>...` — a notification about a terminal, in your words. It
  never changes the terminal's status. Needs nothing.
- `wh read [<target>] [--last <n> | --lines <from>:<to> | --block <id>|last] [--join] [--styles] [--numbers]`
  — what a terminal shows, as text. The screen by default (a full-screen program's too); `--last 200`
  the last 200 rows of history and screen; `--lines -50:-1` grid lines (0 is the top of the screen,
  history is negative); `--block last` the newest command with its output; `--join` puts wrapped rows
  back together. `--json` also has the size, the cursor, `screen` (`alt` while a full-screen program
  runs) and the keyboard and mouse modes, and with `--styles` a `styles` list with one entry per line: the
  runs of that line that are not in the default style, each `{"col","len","fg","bg","bold","italic",
  "underline","inverse","dim"}` (colours `#rrggbb` or a palette number; only the set attributes). A
  selected menu entry or a cursor bar shows as `inverse`. Needs flag, read.
- `wh blocks [<target>] [--last <n>] [--output]` — the shell's commands: id, state, exit code, time;
  `--output` what each printed. Each has `took_ms` (`run` says `duration_ms`). Needs flag, read, and a shell with integration.
- `wh events [--topics <list>] [--terminal <id>]... [--window <id>]... [--output]` — a stream of what
  happens (topics: windows, terminals, status, title, folder, bell, blocks, splits, output). It does
  not return: use `wait` in a tool call, `events` where a process can run in the background. Needs flag;
  `--output` also read.
- `wh wait <target> [--status <state>[,<state>]...] [--match <regex>] [--gone <regex>] [--stable <ms>] [--block-done [--after <block>]] [--timeout <s>]`
  — returns when the first condition holds and says which (`met`). Default 30 s, at most 600; exit 124
  when the time is up. `--match` and `--gone` read the screen (read); the others need nothing more
  than the flag. Needs flag.

## Doing

- `wh run <target> [--timeout <s>] [--exit-code] ['<command>' | -- <word>...]` — types the command at
  the shell's prompt, waits for it to finish, returns `output`, `exit_code`, `duration_ms`, `block`.
  The reply is ok whatever the exit code; `--exit-code` makes the process exit with it instead (1, 2
  and 124 then also mean refusal, bad arguments and timeout: read `--json` when it matters). 10 minutes
  by default, at most 3600. Needs flag, write, shell integration, the shell at its prompt, and a
  terminal that is not yours. The output needs read.
- `wh send-keys <target> <key>... [--type <string>]... [--paste <string>]...` — presses keys in order:
  `enter`, `tab`, `escape`, `up`, `down`, `left`, `right`, `home`, `end`, `pageup`, `pagedown`, `f1` to
  `f24`, `ctrl-c`, `alt-x`, `shift-tab`, or one character. `--type` types a string key by key,
  `--paste` pastes it. Everything goes out in the order written, keys and strings mixed. At most 64 items
  and 4 KiB. Needs flag, write.
- `wh send-text <target> [--submit] <text>... | -` — types text as a paste; `--submit` presses Return;
  `-` reads stdin. At most 16 KiB. Needs write.
- `wh mouse <target> click|press|release|move <row> <col> | scroll <row> <col> up|down|left|right` —
  `--button left|middle|right`, `--double`, `--shift`, `--alt`, `--ctrl`, `--by <n>` for the wheel. Only
  for a program that asked for the mouse (else `refused`, `no_mouse`). Rows and columns count from 0.
  Needs flag, write.
- `wh new-terminal [--folder <path>] [--command <command>] [--window <id>] [--worktree <branch>]` — a
  terminal in the front window. `--command` is typed at the new shell's prompt once the shell is ready,
  after the reply (see below). `--worktree` only opens a dialog for the user. Needs write.
- `wh new-window [--folder <path>] [--command <command>] [--activate]` — a window with one terminal,
  opened behind the user's other windows without taking their keys unless `--activate`. Returns the ids.
  Needs flag, write.
- `wh split <target> [--right|--down|--left|--up] [--size <percent>] [--folder <path>] [--command <command>] [--select]`
  — a new terminal beside the target, in its folder, selection and keys unchanged unless `--select`.
  Returns `terminal`. `refused` with `no_room` when the window is too small. Needs flag, write.
- `wh focus <target> [--activate]` — selects a terminal; keys stay put unless `--activate`. Needs flag, write.
- `wh rename <target> <name>... | --clear` — names a terminal (64 characters). Needs flag, write.
- `wh close <target>` — asks the user in the window whether to close it. It never closes by itself; the
  reply only says the question is up. Needs flag, write.

## Around the edge

- `wh skill install|uninstall|show [--agent <id>] [--path] [<file>]` — this skill: installs into coding
  agents' skill folders on the user's say, removes it, or shows it. Works without the app. Do not run
  `install` unasked: the user chooses it in Wormhole's settings or by running it.
- `wh sessions list`, `wh sessions end <id>` or `wh sessions end --all`, `wh sessions forget <id>` — terminals that
  keep running behind the app (persistent terminals). Works without the app; `end` ends a user's terminal,
  so do not run it unasked.
- `wh mcp` — serves these verbs as MCP tools on stdin and stdout; the agent's configuration starts it
  (see [mcp.md](mcp.md)); it is not for typing.
- `wh welcome` — the first-run welcome for the user; not for agents.
- `wh help` and `wh help --json` — the usage; the verbs, flags, exit codes and error codes as data.

## `--command` of `new-terminal`, `new-window` and `split`

The reply returns as soon as the terminal exists. The command is typed into its shell, and run, once the
shell shows its first prompt (told by the shell's own mark, else when its output goes quiet, at most a few
seconds): it is not an argument of the shell and nothing replaces the shell. A terminal whose shell
cannot take typed input (a program that reads nothing) never runs it. To know it started: `wh wait
--terminal 9 --match '<something the command prints>'`, `wh read --terminal 9 --last 20`, or `wh blocks
--terminal 9 --last 1` (state `running`; needs shell integration and read).

## Keys and patterns

- Patterns (`--match`, `--gone`) are Rust regular expressions on the screen's lines; a line that wrapped
  is one line; trailing spaces are not part of a line, so a prompt `$ ` is the line `$` and matches
  `'\$$'`. Wait for what a command will print, not for its absence.
- Typed text is printable characters only; name control keys (`ctrl-c`, `enter`). `cmd` is refused.
- Keys are encoded for what the program asked for at that moment. A key that switches modes
  (entering a full-screen program) should end a request; read before you go on.
