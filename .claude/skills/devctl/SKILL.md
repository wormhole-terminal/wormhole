---
name: devctl
description: Build, run and drive an isolated Wormhole instance to look at the UI. Use when asked to take a screenshot, run the app, check the UI, see how a change looks, reproduce a rendering or interaction bug, or verify behaviour in the real app instead of only in tests (macOS only).
allowed-tools: Bash(cargo build:*), Bash(printf:*), Bash(cat:*), Bash(mkdir:*), Bash(cp:*), Bash(kill:*), Bash(sleep:*)
---

# Drive an isolated Wormhole with devctl

`devctl` (cargo feature `devctl`) lets you script a running Wormhole through a command file:
screenshots, window state, typed input, clicks, settings and Studio. It needs macOS and a display;
it cannot run in a Linux container. The command list and syntax are the header of
`crates/wormhole/src/devctl/mod.rs`; read it for anything not shown here.

## Safety rules

- Never touch a Wormhole you did not start. The user may have the real app or `scripts/dev.sh` running.
  Stop only your own process, by the PID you captured. No `pkill`, `killall` or name matching.
- Never point the instance at the user's data dir, repos or shell. Use a scratch dir and a throwaway repo.
- Do not delete with globs (`rm *`). Give every artifact a unique name instead of cleaning up.
- Do not trigger the Touch ID prompt (`WORMHOLE_AUTH=allow|deny|cancel` answers it in devctl builds).

## Run

Build into a target dir of your own so a parallel build cannot replace the binary, then start the
instance with `scripts/devctl-instance.sh`, which copies the binary out before running it (as
`scripts/dev.sh` does). In a shared `CARGO_TARGET_DIR`, `debug/Wormhole` is whatever any worktree
built last, and a rebuild would replace the file under the running process.

```sh
S=<scratchpad dir>/hd-$(date +%s)            # unique per run, outside the repository
export CARGO_TARGET_DIR=<target dir for this worktree>
cargo build -p wormhole -p wormhole-host --features wormhole/devctl   # the host sits beside the app
scripts/devctl-instance.sh "$S" <throwaway git repo>   # prints ctl=$S/ctl and pid=<pid>
printf 'onboarding close\nappearance dark\n' > "$S/ctl/cmd"   # a fresh data dir is a first launch
```

`scripts/devctl-instance.sh [--bin <path>] [--shell <path>] [--env KEY=VALUE] <scratch dir> [start dir]`
keeps everything under the scratch dir and starts the copy with:

- `WORMHOLE_BACKGROUND=1`: agents' test instances open behind your windows and never take focus.
  The app never activates itself and has no Dock icon; every window (a second one, Settings, the
  Studio, About, the quick panel) opens without the focus, behind the other apps' windows. Covered
  windows are still drawn, so `shot`, clicks and `wait-idle` work as on a visible window. Only when a
  check truly needs the keys, devctl's `front` brings the window forward; don't use it otherwise.
- `WORMHOLE_DATA_DIR=$S/data`: scratch database, config, logs, hook socket. Start each run with a new
  scratch dir: a database from an older build is upgraded in place, and one from before migrations is
  moved aside (`wormhole.db.old-<time>`).
- `HOME=$S/home`: no user git config, and paths read `~/…`.
- `WORMHOLE_SHELL=/bin/cat`: typed text is echoed, never executed. To test agent detection, pass
  `--shell` a copy of `/bin/cat` named like an agent (`codex`, `claude`, ...).
- `WORMHOLE_START_DIR`: the folder the first terminal opens in (default: the scratch `HOME`); use a
  throwaway repo (create one with `git init` in the scratchpad) for Changes, History and diff views.
- `WORMHOLE_DEVCTL=$S/ctl`: the control directory. `WORMHOLE_AGENT_HOME=$S/agent-home`: where
  Settings -> Coding agents installs hooks (instead of `~/.claude`, `~/.codex`, ...).
- `PATH=/usr/bin:/bin:/usr/sbin:/sbin` (the Finder's) and `WORMHOLE_AGENT_PATH=` (empty): no agent
  counts as installed, whatever the Mac has. To show one, `--env WORMHOLE_AGENT_PATH=<dir>` with a
  copy of `/bin/cat` named like the agent in `<dir>` (the visual check does this for `@shell`). A
  terminal's shell gets none of the instance's own `WORMHOLE_*` variables.
- `WORMHOLE_AUTH=cancel`: Touch ID is answered without its prompt. `WORMHOLE_NOTIFY=log`:
  notifications are kept for `notify-log`, never shown.
- Background mode also keeps the instance out of the menu bar, the Dock (no badge or bounce), the
  bell's sound, VoiceOver and the global hotkey; `menubar`, `dock` and `a11y said` show what they would
  have been. An instance started by hand without it shows all of that until it quits.

## Send commands

One command per line into `$S/ctl/cmd`; the app consumes the file on its next tick (about 400 ms) and
writes results to `$S/ctl/out.txt`. Screenshots land in `$S/ctl/<name>.png` (view them with Read).

```sh
printf 'resize 1200 800\nnew\n' > "$S/ctl/cmd"     # a change ...
sleep 1
printf 'frames 2\n' > "$S/ctl/cmd"                  # ... let it render ...
sleep 1
printf 'shot home\nstate\n' > "$S/ctl/cmd"          # ... then capture
sleep 1; cat "$S/ctl/out.txt"
```

- `shot` renders the last drawn frame. Send `frames 2` (redraw twice) in an earlier batch than the
  change you want to see, and `shot` in a later one, or the picture shows the state before the change.
- A batch's `out.txt` is overwritten by the next batch; read it before sending more.
- `resize <w> <h>` sets the window size in logical pixels (window minimum still applies); coordinates
  for `click`/`hover` are logical pixels in that window.
- Screenshots are GPUI's own render: the native window blur is not in them.
- Speed: `frames <n>` records each forced draw's time in `frames.txt` (the same whether or not the
  window can be seen); `paints` reports terminal grids drawn per second, which only means something
  while the window is on screen (macOS throttles a hidden one to ~20). `scripts/perf.sh` runs both
  under ttystorm's loads.
- Window N reads `cmdN` and writes `outN.txt` (`cmd2`/`out2.txt` for the second window).
- The Studio and settings windows open on the next tick: send their follow-up commands in a later batch.
- Debug builds: `inspect` (⇧⌥⌘I), `hover x y`, `click x y` open the inspector and report an element's file:line
  and spacing/radius/text, which is the quickest way to find what to change.
- Common commands: `state` (its last line says where the keyboard is: focus terminal, focus
  sidebar, …), `screen`, `type`, `key ctrl-cmd-2` (any key binding, F6 too: `key f6`; shifted
  punctuation as `key shift-cmd-]`), `menus` (the menu bar with its keys),
  `new`, `rows` (the sidebar's list with each row's split marks), `pane-split right|down`, `split-join`,
  `split-leave`, `row-drop`, `note`, `task`, `changes`,
  `diff <n>`, `history`, `compare`, `double-click <x> <y>`, `settings-section <name>`, `settings-shot`,
  `settings-search [text]`, `settings-select <row>`, `settings-record <id> [keys]`,
  `settings-preview <density>`, `settings-scroll <px>`, `settings-resize <w> <h>`, `settings-reset-confirm`,
  `onboarding <step|close>` (fresh data dirs open the terminal welcome, a program in the first terminal, or the intro window with its flag off; `onboarding close` first skips either), `welcome <cursor|portal|logo> [hidden]` (replay the terminal welcome with the start animation's hand-over), `welcome-dock [name]` (the ship where it docks, over the window's picture), `welcome-probe <tab|-> <token> <op JSON>` (a request as the program sends it; `state` lists each welcome's token, its example band and how often macOS was asked), `notify-permission <not-asked|allowed|denied|outside|checking>` (pretend what macOS said, for the welcome's notifications step),
  `palette [query]` / `palette close` (⇧⌘P; prefixes `@ ~ # , >`), `sheet [filter]` / `sheet close` (⌥⌘/),
  `appearance dark` (under Match system, the default, a Mac in Light Mode shows the light pick),
  `studio`, `studio-shot`, `studio-select <token>`, `studio-set <value>` (typed into the token editor's
  field, Return pressed: an expression that doesn't work keeps its error), `studio-filter <all|changed|low>`,
  `studio-area <area>`, `studio-menu [save|copy|json|project]`, `set key=value` (JSON value), `quit`.
- The sidebar: `new ~/dir` (a terminal in another folder), `rename <n> <name>`,
  `group-new <name> <row>…` (a group of those terminals), `sidebar-fold <group>`, `pin <n>`,
  `folder-color [path] <name|none>` (a folder's colour: its terminals' title row is tinted; written to
  the project's `config.json` as `project_color` when the folder's project has one, else to Wormhole's
  settings), `hold-cmd` / `release-cmd` (the ⌘1–9 keycaps show 250 ms after
  `hold-cmd`: `shot` in a later batch), `right-click <x> <y>` (a row's or a group's menu),
  `sidebar-scroll <px>`. Context menus are native macOS menus: no `shot` shows them and no `click` reaches
  them, and a background instance doesn't pop them up at all. After a `right-click`, `menu` lists the menu
  that opened (one row a line, items numbered), `menu-pick <n|title>` chooses an item as a click would and
  `menu-close` dismisses it (the row's focus ring goes with it).
- The right panel: `panel-tab notes|changes|history|project`, `project-init` (the Project tab's Create project
  folder button, for the active terminal's folder), `task-seed <open> <done>` and
  `task-done <n>` (next steps; the done ones are spread among the open ones), `note-add <text>`,
  `note-toggle <n>` (open or fold the n-th note), `note-seed <count> <lines>` (long notes),
  `picker compare|history [query]` (the branch select) and `panel-scroll <px>` (moves the list's
  state by pixels; it cannot get past rows that were never measured). `scroll <x> <y> <dy>` turns
  the wheel over a point as a trackpad would, a frame between its steps, so it reaches the end of
  any list (the sidebar's, the panel's, the diff) and the rest of the batch waits for it.
- `set key=value` writes `config.json`, which the app reads back through a file watcher: wait about a
  second before the `shot` that should show it (in a scenario, `@sleep 1500`).
- The status bar is off by default: `set status_bar=true` shows it (`set status_bar=false` hides it
  again). Turn it on before a `shot` that should show it.
- `bounds` may answer "nothing at" for a point in a 1000-wide window while the same point works at
  1440: `resize 1440 900` first when you only want to measure an element.
- A synthetic `hover` needs two moves before an element sees the pointer (hover fills, tooltips, a
  resize handle's line): send two `hover` lines, a pixel apart, then `shot` in a later batch.

## Visual check

To compare a change with main, let `scripts/visual-check.sh` do all of the
above: it builds once, copies the binary out, runs each scenario in its own isolated instance, stops it
by PID and makes the screenshots deterministic (two runs are pixel-identical).

```sh
scripts/visual-check.sh home changes-diff --out <scratchpad dir>/vc-1
scripts/visual-check.sh all --against origin/main --out <scratchpad dir>/vc-2   # diffed against main
```

- Scenarios are `tools/visual/scenarios/*.txt` (devctl lines plus directives such as `@fixture`,
  `@wait-idle`, `@shot`), fixtures `tools/visual/fixtures/*.sh`; the format is in
  `tools/visual/README.md`. Add a scenario for the UI you change.
- Read `results.txt` and the `diff/*-side.png` pictures (1x); list the shots you looked at in the PR.
- The same pieces by hand: `wait-idle` (the rest of the batch waits until no git work is in flight and
  the windows stopped changing, so `shot` can follow in the same batch), `sync <token>` (written last to
  `out.txt` once the batch is done: wait for it instead of sleeping), `appearance light|dark|system` and
  `reduce-transparency on|off|system` (stand in for macOS's Light/Dark Mode, which Match system follows,
  and Reduce Transparency; never change the Mac's own settings), `increase-contrast on|off|system` (the
  theme's high-contrast tokens, opaque windows, a 3 pt ring), `reduce-motion on|off|system` (animations
  held still: still dots, a steady halo, cuts for the slides), `hook <tab> <event> [message]` (agent states without an agent; use a shell named like the agent
  so they last), `bounds <x> <y> [levels]` (an element's bounds in logical px, for x 16/40/334 and
  heights 28/32/40/44).
- Keyboard and VoiceOver: `key f6` / `key tab` / `key down` move the keys (each `key` is
  dispatched after its batch, so read `state` in a later one: its last line names the area, the item the
  keys are on, and says ring while the ring shows). The ring hides in a window that isn't in front, which a
  scripted instance never is: `window-front on` says it is. `keyboard-navigation on` makes the title
  row's buttons ⇥ stops. `a11y` wakes the accessibility tree (GPUI builds it only for an assistive app)
  and prints it a node a line, `*` on the one VoiceOver's focus is on; `a11y said` lists the
  announcements made lately (`!` for one that interrupts). Never start VoiceOver for a check.
- A copy of `/bin/cat` that is killed at launch (exit 137) runs once re-signed: `codesign -s - -f <copy>`.

## Quit and confirm

```sh
scripts/devctl-instance.sh stop "$S"   # `quit`, then TERM only the PID in $S/pid if it ignored that
```

Confirm with `kill -0 <pid>` (it must fail) and tell the user the screenshot paths. If the app died
early, `$S/app.log` and the log in `$S/data/logs/` say why.

## When you cannot run it

On Linux, in a container, or without a display, do not pretend: report that the change affects
rendering and was verified by tests only, and say which screenshot would settle it.
